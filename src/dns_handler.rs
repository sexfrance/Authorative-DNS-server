use crate::{DnsConfig, DomainManager, domain_manager::VerificationStatus};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use trust_dns_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use trust_dns_proto::rr::{Name, RData, Record, RecordType};
use trust_dns_proto::serialize::binary::{BinDecodable, BinEncodable, BinEncoder};

#[derive(Clone)]
pub struct CybertempHandler {
    config: DnsConfig,
    domain_manager: Arc<RwLock<DomainManager>>,
}

impl CybertempHandler {
    pub fn new(config: DnsConfig, domain_manager: Arc<RwLock<DomainManager>>) -> Self {
        Self {
            config,
            domain_manager,
        }
    }
    
    pub async fn handle_request(&self, data: &[u8]) -> Result<Vec<u8>> {
        let request = Message::from_bytes(data)?;
        
        let response = self.handle_dns_message(request).await?;
        
        let mut response_data = Vec::new();
        let mut encoder = BinEncoder::new(&mut response_data);
        response.emit(&mut encoder)?;
        
        Ok(response_data)
    }
    
    async fn handle_dns_message(&self, request: Message) -> Result<Message> {
        let mut response = Message::new();
        response.set_id(request.id());
        response.set_op_code(request.op_code());
        response.set_message_type(MessageType::Response);
        response.set_recursion_desired(request.recursion_desired());
        
        if request.op_code() != OpCode::Query {
            response.set_response_code(ResponseCode::NotImp);
            return Ok(response);
        }
        
        for query in request.queries() {
            self.handle_query(query, &mut response).await;
        }
        
        Ok(response)
    }
    
    async fn handle_query(&self, query: &Query, response: &mut Message) {
        let name = query.name().to_ascii();
        let query_type = query.query_type();
        
        tracing::debug!("DNS query: {} type: {:?}", name, query_type);
        
        match query_type {
            RecordType::A => self.handle_a_record(&name, response).await,
            RecordType::MX => self.handle_mx_record(&name, response).await,
            RecordType::TXT => self.handle_txt_record(&name, response).await,
            RecordType::NS => self.handle_ns_record(&name, response).await,
            RecordType::AAAA => self.handle_aaaa_record(&name, response).await,
            _ => {
                response.set_response_code(ResponseCode::NoError);
            }
        }
    }
    
    async fn handle_a_record(&self, domain: &str, response: &mut Message) {
        let manager = self.domain_manager.read().await;
        
        if let Some(record) = manager.get_domain(domain).await {
            if !record.enabled || record.verification_status != VerificationStatus::Verified {
                response.set_response_code(ResponseCode::Refused);
                return;
            }
            
            // Use the IP from the domain record (which could be Discord IP)
            if let Ok(ip) = record.ip.parse::<std::net::Ipv4Addr>() {
                let name = Name::from_ascii(domain).unwrap();
                let dns_record = Record::from_rdata(
                    name,
                    self.config.default_ttl,
                    RData::A(ip.into()),
                );
                response.add_answer(dns_record);
            }
            
            // Handle mail subdomain with appropriate IP
            if domain.starts_with("mail.") || domain == "mail" {
                let base_domain = if domain == "mail" {
                    // This is for mail.cybertemp.xyz etc
                    "cybertemp.xyz"
                } else {
                    domain.trim_start_matches("mail.")
                };
                
                if let Some(parent_record) = manager.get_domain(base_domain).await {
                    let mail_ip = if parent_record.discord {
                        "37.114.41.81"
                    } else {
                        "45.134.39.50"
                    };
                    
                    if let Ok(ip) = mail_ip.parse::<std::net::Ipv4Addr>() {
                        let name = Name::from_ascii(domain).unwrap();
                        let dns_record = Record::from_rdata(
                            name,
                            self.config.default_ttl,
                            RData::A(ip.into()),
                        );
                        response.add_answer(dns_record);
                    }
                }
            }
        }
        
        response.set_response_code(ResponseCode::NoError);
    }
    
    async fn handle_mx_record(&self, domain: &str, response: &mut Message) {
        let manager = self.domain_manager.read().await;
        
        if let Some(record) = manager.get_domain(domain).await {
            if !record.enabled || record.verification_status != VerificationStatus::Verified {
                response.set_response_code(ResponseCode::Refused);
                return;
            }
            
            let name = Name::from_ascii(domain).unwrap();
            
            // Create appropriate mail server name based on Discord flag
            let mail_server = if record.discord {
                format!("mail.{}.discord.cybertemp.xyz", domain)
            } else {
                self.config.mail_server.replace("{domain}", domain)
            };
            
            let mx_name = Name::from_ascii(&mail_server).unwrap();
            
            // Main MX record
            let mx_record = Record::from_rdata(
                name.clone(),
                self.config.default_ttl,
                RData::MX(trust_dns_proto::rr::rdata::MX::new(
                    self.config.mx_priority,
                    mx_name.clone(),
                )),
            );
            response.add_answer(mx_record);
            
            // Wildcard MX record
            let wildcard_name = Name::from_ascii(&format!("*.{}", domain)).unwrap();
            let wildcard_mx_record = Record::from_rdata(
                wildcard_name,
                self.config.default_ttl,
                RData::MX(trust_dns_proto::rr::rdata::MX::new(
                    self.config.mx_priority,
                    mx_name,
                )),
            );
            response.add_answer(wildcard_mx_record);
        }
        
        response.set_response_code(ResponseCode::NoError);
    }
    
    async fn handle_txt_record(&self, domain: &str, response: &mut Message) {
        let manager = self.domain_manager.read().await;
        
        if let Some(record) = manager.get_domain(domain).await {
            if !record.enabled || record.verification_status != VerificationStatus::Verified {
                response.set_response_code(ResponseCode::Refused);
                return;
            }
            
            let name = Name::from_ascii(domain).unwrap();
            
            // SPF record
            let spf_record = Record::from_rdata(
                name.clone(),
                self.config.default_ttl,
                RData::TXT(trust_dns_proto::rr::rdata::TXT::new(vec![
                    "v=spf1 a mx include:_spf.google.com -all".to_string(),
                ])),
            );
            response.add_answer(spf_record);
            
            // DMARC record
            let dmarc_name = Name::from_ascii(&format!("_dmarc.{}", domain)).unwrap();
            let dmarc_record = Record::from_rdata(
                dmarc_name,
                self.config.default_ttl,
                RData::TXT(trust_dns_proto::rr::rdata::TXT::new(vec![
                    "v=DMARC1; p=none;".to_string(),
                ])),
            );
            response.add_answer(dmarc_record);
        }
        
        response.set_response_code(ResponseCode::NoError);
    }
    
    async fn handle_ns_record(&self, domain: &str, response: &mut Message) {
        let manager = self.domain_manager.read().await;
        
        if let Some(record) = manager.get_domain(domain).await {
            if !record.enabled || record.verification_status != VerificationStatus::Verified {
                response.set_response_code(ResponseCode::Refused);
                return;
            }
            
            let name = Name::from_ascii(domain).unwrap();
            
            for ns in &self.config.nameservers {
                let ns_record = Record::from_rdata(
                    name.clone(),
                    self.config.default_ttl,
                    RData::NS(trust_dns_proto::rr::rdata::NS(Name::from_ascii(ns).unwrap())),
                );
                response.add_answer(ns_record);
            }
        }
        
        response.set_response_code(ResponseCode::NoError);
    }
    
    async fn handle_aaaa_record(&self, _domain: &str, response: &mut Message) {
        response.set_response_code(ResponseCode::NoError);
    }
}