"""
VexFS v2 Qdrant Adapter - Production Security

Enterprise-grade security implementation for production deployment.
"""

import jwt
import time
import hashlib
import secrets
from typing import Dict, List, Any, Optional, Callable
from dataclasses import dataclass
import logging
from datetime import datetime, timedelta
import asyncio
import threading
from collections import defaultdict

logger = logging.getLogger(__name__)

@dataclass
class SecurityConfig:
    """Security configuration"""
    jwt_secret_key: str = None
    jwt_algorithm: str = "HS256"
    jwt_expiration_hours: int = 24
    api_key_length: int = 32
    rate_limit_requests: int = 1000
    rate_limit_window: int = 3600  # 1 hour
    max_failed_attempts: int = 5
    lockout_duration: int = 900  # 15 minutes

class ProductionSecurity:
    """
    Enterprise-grade security implementation for VexFS v2 Qdrant adapter.
    
    Features:
    - JWT token validation
    - API key management
    - Role-based access control
    - Rate limiting
    - Session management
    - Audit logging
    """
    
    def __init__(self, config: SecurityConfig = None):
        self.config = config or SecurityConfig()
        
        # Generate secret key if not provided
        if not self.config.jwt_secret_key:
            self.config.jwt_secret_key = secrets.token_urlsafe(32)
        
        # Authentication components
        self.auth_manager = AuthenticationManager(self.config)
        self.access_control = AccessControlManager()
        self.rate_limiter = RateLimiter(self.config)
        self.session_manager = SessionManager(self.config)
        self.audit_logger = AuditLogger()
        
        logger.info("🔒 Production security initialized")
    
    async def authenticate_request(self, request_headers: Dict[str, str], 
                                 client_ip: str = None) -> Optional[Dict[str, Any]]:
        """Authenticate incoming request"""
        try:
            # Check rate limiting first
            if client_ip and not await self.rate_limiter.check_rate_limit(client_ip):
                self.audit_logger.log_security_event("rate_limit_exceeded", {"client_ip": client_ip})
                return None
            
            # Try JWT authentication
            auth_header = request_headers.get("Authorization", "")
            if auth_header.startswith("Bearer "):
                token = auth_header[7:]
                user_info = await self.auth_manager.validate_jwt_token(token)
                if user_info:
                    self.audit_logger.log_security_event("jwt_auth_success", {"user": user_info.get("user_id")})
                    return user_info
            
            # Try API key authentication
            api_key = request_headers.get("X-API-Key") or request_headers.get("Api-Key")
            if api_key:
                user_info = await self.auth_manager.validate_api_key(api_key)
                if user_info:
                    self.audit_logger.log_security_event("api_key_auth_success", {"user": user_info.get("user_id")})
                    return user_info
            
            # Authentication failed
            self.audit_logger.log_security_event("auth_failed", {"client_ip": client_ip})
            return None
            
        except Exception as e:
            logger.error(f"Authentication error: {e}")
            self.audit_logger.log_security_event("auth_error", {"error": str(e), "client_ip": client_ip})
            return None
    
    async def authorize_request(self, user_info: Dict[str, Any], 
                              resource: str, action: str) -> bool:
        """Authorize request based on user permissions"""
        try:
            return await self.access_control.check_permission(user_info, resource, action)
        except Exception as e:
            logger.error(f"Authorization error: {e}")
            return False
    
    async def create_session(self, user_info: Dict[str, Any]) -> str:
        """Create user session"""
        return await self.session_manager.create_session(user_info)
    
    async def validate_session(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Validate user session"""
        return await self.session_manager.validate_session(session_id)
    
    def get_security_stats(self) -> Dict[str, Any]:
        """Get security statistics"""
        return {
            "active_sessions": self.session_manager.get_active_session_count(),
            "rate_limit_stats": self.rate_limiter.get_stats(),
            "auth_stats": self.auth_manager.get_stats(),
            "audit_events_today": self.audit_logger.get_events_count_today(),
            "security_alerts": self.audit_logger.get_recent_alerts()
        }

class AuthenticationManager:
    """JWT and API key authentication management"""
    
    def __init__(self, config: SecurityConfig):
        self.config = config
        self.api_keys = {}  # In production, this would be a database
        self.failed_attempts = defaultdict(int)
        self.lockouts = {}
        self.stats = {
            "jwt_validations": 0,
            "api_key_validations": 0,
            "failed_attempts": 0
        }
    
    async def validate_jwt_token(self, token: str) -> Optional[Dict[str, Any]]:
        """Validate JWT token"""
        try:
            payload = jwt.decode(
                token, 
                self.config.jwt_secret_key, 
                algorithms=[self.config.jwt_algorithm]
            )
            
            # Check expiration
            if payload.get("exp", 0) < time.time():
                return None
            
            self.stats["jwt_validations"] += 1
            return payload
            
        except jwt.InvalidTokenError:
            self.stats["failed_attempts"] += 1
            return None
    
    async def validate_api_key(self, api_key: str) -> Optional[Dict[str, Any]]:
        """Validate API key"""
        try:
            # Hash the API key for lookup
            key_hash = hashlib.sha256(api_key.encode()).hexdigest()
            
            if key_hash in self.api_keys:
                user_info = self.api_keys[key_hash]
                
                # Check if key is active
                if user_info.get("active", True):
                    self.stats["api_key_validations"] += 1
                    return user_info
            
            self.stats["failed_attempts"] += 1
            return None
            
        except Exception:
            self.stats["failed_attempts"] += 1
            return None
    
    def create_jwt_token(self, user_info: Dict[str, Any]) -> str:
        """Create JWT token"""
        payload = {
            **user_info,
            "exp": time.time() + (self.config.jwt_expiration_hours * 3600),
            "iat": time.time()
        }
        
        return jwt.encode(payload, self.config.jwt_secret_key, algorithm=self.config.jwt_algorithm)
    
    def create_api_key(self, user_info: Dict[str, Any]) -> str:
        """Create new API key"""
        api_key = secrets.token_urlsafe(self.config.api_key_length)
        key_hash = hashlib.sha256(api_key.encode()).hexdigest()
        
        self.api_keys[key_hash] = {
            **user_info,
            "created_at": time.time(),
            "active": True
        }
        
        return api_key
    
    def revoke_api_key(self, api_key: str) -> bool:
        """Revoke API key"""
        key_hash = hashlib.sha256(api_key.encode()).hexdigest()
        if key_hash in self.api_keys:
            self.api_keys[key_hash]["active"] = False
            return True
        return False
    
    def get_stats(self) -> Dict[str, Any]:
        """Get authentication statistics"""
        return self.stats.copy()

class AccessControlManager:
    """Role-based access control management"""
    
    def __init__(self):
        self.roles = {
            "admin": {
                "permissions": ["*"]  # All permissions
            },
            "user": {
                "permissions": [
                    "collections:read",
                    "collections:write",
                    "points:read",
                    "points:write",
                    "search:execute"
                ]
            },
            "readonly": {
                "permissions": [
                    "collections:read",
                    "points:read",
                    "search:execute"
                ]
            }
        }
    
    async def check_permission(self, user_info: Dict[str, Any], 
                             resource: str, action: str) -> bool:
        """Check if user has permission for resource and action"""
        user_role = user_info.get("role", "readonly")
        
        if user_role not in self.roles:
            return False
        
        permissions = self.roles[user_role]["permissions"]
        
        # Check for wildcard permission
        if "*" in permissions:
            return True
        
        # Check for specific permission
        required_permission = f"{resource}:{action}"
        return required_permission in permissions
    
    def add_role(self, role_name: str, permissions: List[str]):
        """Add new role with permissions"""
        self.roles[role_name] = {"permissions": permissions}
    
    def update_role_permissions(self, role_name: str, permissions: List[str]):
        """Update role permissions"""
        if role_name in self.roles:
            self.roles[role_name]["permissions"] = permissions

class RateLimiter:
    """Rate limiting implementation"""
    
    def __init__(self, config: SecurityConfig):
        self.config = config
        self.requests = defaultdict(list)
        self.lock = threading.RLock()
        self.stats = {
            "total_requests": 0,
            "blocked_requests": 0,
            "unique_clients": 0
        }
    
    async def check_rate_limit(self, client_id: str) -> bool:
        """Check if client is within rate limits"""
        with self.lock:
            current_time = time.time()
            window_start = current_time - self.config.rate_limit_window
            
            # Clean old requests
            self.requests[client_id] = [
                req_time for req_time in self.requests[client_id]
                if req_time > window_start
            ]
            
            # Check rate limit
            if len(self.requests[client_id]) >= self.config.rate_limit_requests:
                self.stats["blocked_requests"] += 1
                return False
            
            # Add current request
            self.requests[client_id].append(current_time)
            self.stats["total_requests"] += 1
            
            # Update unique clients count
            self.stats["unique_clients"] = len(self.requests)
            
            return True
    
    def get_stats(self) -> Dict[str, Any]:
        """Get rate limiting statistics"""
        return self.stats.copy()

class SessionManager:
    """User session management"""
    
    def __init__(self, config: SecurityConfig):
        self.config = config
        self.sessions = {}
        self.lock = threading.RLock()
    
    async def create_session(self, user_info: Dict[str, Any]) -> str:
        """Create new user session"""
        session_id = secrets.token_urlsafe(32)
        
        with self.lock:
            self.sessions[session_id] = {
                "user_info": user_info,
                "created_at": time.time(),
                "last_accessed": time.time()
            }
        
        return session_id
    
    async def validate_session(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Validate and refresh session"""
        with self.lock:
            if session_id not in self.sessions:
                return None
            
            session = self.sessions[session_id]
            current_time = time.time()
            
            # Check if session expired
            if current_time - session["last_accessed"] > (self.config.jwt_expiration_hours * 3600):
                del self.sessions[session_id]
                return None
            
            # Update last accessed time
            session["last_accessed"] = current_time
            return session["user_info"]
    
    async def revoke_session(self, session_id: str) -> bool:
        """Revoke user session"""
        with self.lock:
            if session_id in self.sessions:
                del self.sessions[session_id]
                return True
            return False
    
    def get_active_session_count(self) -> int:
        """Get number of active sessions"""
        with self.lock:
            return len(self.sessions)
    
    def cleanup_expired_sessions(self):
        """Clean up expired sessions"""
        with self.lock:
            current_time = time.time()
            expired_sessions = [
                session_id for session_id, session in self.sessions.items()
                if current_time - session["last_accessed"] > (self.config.jwt_expiration_hours * 3600)
            ]
            
            for session_id in expired_sessions:
                del self.sessions[session_id]

class AuditLogger:
    """Security audit logging"""
    
    def __init__(self):
        self.events = []
        self.lock = threading.RLock()
        self.max_events = 10000
    
    def log_security_event(self, event_type: str, details: Dict[str, Any]):
        """Log security event"""
        with self.lock:
            event = {
                "timestamp": time.time(),
                "event_type": event_type,
                "details": details,
                "severity": self._get_event_severity(event_type)
            }
            
            self.events.append(event)
            
            # Keep only recent events
            if len(self.events) > self.max_events:
                self.events = self.events[-self.max_events:]
            
            # Log to system logger for critical events
            if event["severity"] in ["high", "critical"]:
                logger.warning(f"Security event: {event_type} - {details}")
    
    def _get_event_severity(self, event_type: str) -> str:
        """Get event severity level"""
        high_severity_events = [
            "rate_limit_exceeded",
            "auth_failed",
            "unauthorized_access",
            "suspicious_activity"
        ]
        
        if event_type in high_severity_events:
            return "high"
        elif "error" in event_type:
            return "medium"
        else:
            return "low"
    
    def get_events_count_today(self) -> int:
        """Get number of events today"""
        today_start = datetime.now().replace(hour=0, minute=0, second=0, microsecond=0).timestamp()
        
        with self.lock:
            return len([
                event for event in self.events
                if event["timestamp"] >= today_start
            ])
    
    def get_recent_alerts(self) -> List[Dict[str, Any]]:
        """Get recent high-severity alerts"""
        with self.lock:
            recent_time = time.time() - 3600  # Last hour
            
            return [
                event for event in self.events
                if event["timestamp"] >= recent_time and event["severity"] in ["high", "critical"]
            ]

# Security middleware for FastAPI
class SecurityMiddleware:
    """Security middleware for request processing"""
    
    def __init__(self, security_manager: ProductionSecurity):
        self.security = security_manager
    
    async def __call__(self, request, call_next):
        """Process request through security middleware"""
        # Extract client IP
        client_ip = request.client.host if hasattr(request, 'client') else None
        
        # Authenticate request
        user_info = await self.security.authenticate_request(
            dict(request.headers), 
            client_ip
        )
        
        if not user_info:
            # Return 401 Unauthorized
            from fastapi import HTTPException
            raise HTTPException(status_code=401, detail="Authentication required")
        
        # Add user info to request state
        request.state.user = user_info
        
        # Process request
        response = await call_next(request)
        
        return response

# Global security instance
_global_security = None

def get_security_manager() -> ProductionSecurity:
    """Get global security manager instance"""
    global _global_security
    if _global_security is None:
        _global_security = ProductionSecurity()
    return _global_security