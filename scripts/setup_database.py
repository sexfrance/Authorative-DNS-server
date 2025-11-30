#!/usr/bin/env python3
import psycopg2
import sys
import os
import dotenv

# Load environment variables from .env file
script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(script_dir)
dotenv_path = os.path.join(project_root, '.env')
dotenv.load_dotenv(dotenv_path=dotenv_path)

def setup_database():
    db_name = os.getenv('DB_NAME', 'dns_server')
    db_user = os.getenv('DB_USER', 'dns_user')
    db_pass = os.getenv('DB_PASS', 'dns_password')
    db_host = os.getenv('DB_HOST', 'localhost')
    db_port = os.getenv('DB_PORT', '5432')
    
    try:
        # Connect to PostgreSQL
        conn = psycopg2.connect(
            host=db_host,
            port=db_port,
            user="postgres",
            password=os.getenv('PGPASSWORD', '')
        )
        conn.autocommit = True
        cursor = conn.cursor()
        
        # Create database and user
        try:
            cursor.execute(f"CREATE DATABASE {db_name};")
        except psycopg2.Error:
            print(f"Database {db_name} already exists")
            
        try:
            cursor.execute(f"CREATE USER {db_user} WITH PASSWORD '{db_pass}';")
        except psycopg2.Error:
            print(f"User {db_user} already exists")
            
        cursor.execute(f"GRANT ALL PRIVILEGES ON DATABASE {db_name} TO {db_user};")
        
        cursor.close()
        conn.close()
        
        # Run migrations
        os.environ['PGPASSWORD'] = db_pass
        conn = psycopg2.connect(
            host=db_host,
            port=db_port,
            user=db_user,
            password=db_pass,
            database=db_name
        )
        cursor = conn.cursor()
        
        with open('migrations/001_initial_schema.sql', 'r') as f:
            cursor.execute(f.read())
            
        conn.commit()
        cursor.close()
        conn.close()
        
        print("Database setup complete!")
        print(f"Database: {db_name}")
        print(f"User: {db_user}")
        
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    setup_database()