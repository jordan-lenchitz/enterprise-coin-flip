import os
from sqlalchemy import create_engine, Column, Integer, String, DECIMAL, ForeignKey, DateTime
from sqlalchemy.orm import declarative_base, sessionmaker, relationship
from datetime import datetime

DATABASE_URL = os.getenv("DATABASE_URL", "postgresql://admin:password@localhost:5432/coinflip")

engine = create_engine(DATABASE_URL)
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

Base = declarative_base()

class Account(Base):
    __tablename__ = "accounts"

    id = Column(Integer, primary_key=True, index=True)
    username = Column(String(255), unique=True, nullable=False)
    password_hash = Column(String(255), nullable=False)
    webhook_url = Column(String(255))
    api_keys = relationship("ApiKey", back_populates="account")
    ledger_entries = relationship("Ledger", back_populates="account")

class ApiKey(Base):
    __tablename__ = "api_keys"

    id = Column(Integer, primary_key=True, index=True)
    account_id = Column(Integer, ForeignKey("accounts.id", ondelete="CASCADE"))
    api_key = Column(String(255), unique=True, nullable=False)
    service = Column(String(255), nullable=False)
    created_at = Column(DateTime, default=datetime.utcnow)
    
    account = relationship("Account", back_populates="api_keys")

class Ledger(Base):
    __tablename__ = "ledger"

    id = Column(Integer, primary_key=True, index=True)
    account_id = Column(Integer, ForeignKey("accounts.id", ondelete="SET NULL"))
    environment = Column(String(50), nullable=False)
    cost = Column(DECIMAL(10, 4), nullable=False)
    result = Column(String(10), nullable=False)
    created_at = Column(DateTime, default=datetime.utcnow)
    
    account = relationship("Account", back_populates="ledger_entries")

# Dependency
def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
