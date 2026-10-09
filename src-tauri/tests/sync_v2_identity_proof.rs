use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const ISSUER: &str = "https://identity.sync-proof.sitedatum.invalid";
const AUDIENCE: &str = "sitedatum-sync-v2";
const REDIRECT_URI: &str = "http://127.0.0.1:43821/callback";
const SUBJECT: &str = "10000000-0000-4000-8000-000000000001";
const DEVICE: &str = "20000000-0000-4000-8000-000000000001";

#[derive(Clone)]
struct PendingCode {
    subject: String,
    redirect_uri: String,
    challenge: String,
    nonce: String,
    device_id: String,
    expires_at: u64,
    used: bool,
}

#[derive(Clone)]
struct Session {
    subject: String,
    device_id: String,
    revoked: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Header {
    alg: String,
    kid: String,
    typ: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    sub: String,
    nonce: String,
    iat: u64,
    exp: u64,
    session_id: String,
    sync_device_id: String,
}

struct TokenBundle {
    id_token: String,
    session_id: String,
}

struct Callback {
    code: String,
    state: String,
}

struct IdentityIssuer {
    now: u64,
    active_kid: String,
    signing_keys: HashMap<String, SigningKey>,
    codes: HashMap<String, PendingCode>,
    sessions: HashMap<String, Session>,
    revoked_devices: HashSet<String>,
    deleted_subjects: HashSet<String>,
    serial: u64,
}

impl IdentityIssuer {
    fn new() -> Self {
        let active_kid = "proof-key-1".to_string();
        let mut signing_keys = HashMap::new();
        signing_keys.insert(active_kid.clone(), SigningKey::from_bytes(&[7_u8; 32]));
        Self {
            now: 2_000_000_000,
            active_kid,
            signing_keys,
            codes: HashMap::new(),
            sessions: HashMap::new(),
            revoked_devices: HashSet::new(),
            deleted_subjects: HashSet::new(),
            serial: 0,
        }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.serial += 1;
        format!("{prefix}-{}", self.serial)
    }

    fn authorize(
        &mut self,
        subject: &str,
        redirect_uri: &str,
        state: &str,
        nonce: &str,
        challenge: &str,
        device_id: &str,
    ) -> Callback {
        let code = self.next_id("code");
        self.codes.insert(
            code.clone(),
            PendingCode {
                subject: subject.to_string(),
                redirect_uri: redirect_uri.to_string(),
                challenge: challenge.to_string(),
                nonce: nonce.to_string(),
                device_id: device_id.to_string(),
                expires_at: self.now + 600,
                used: false,
            },
        );
        Callback {
            code,
            state: state.to_string(),
        }
    }

    fn exchange(
        &mut self,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
    ) -> Result<TokenBundle, &'static str> {
        let pending = self.codes.get_mut(code).ok_or("AUTH_CODE_UNKNOWN")?;
        if self.deleted_subjects.contains(&pending.subject) {
            return Err("ACCOUNT_DELETED");
        }
        if pending.used {
            return Err("AUTH_CODE_REPLAYED");
        }
        // OAuth-style expiry is exclusive: the code is invalid at exp.
        if pending.expires_at <= self.now {
            return Err("AUTH_CODE_EXPIRED");
        }
        if pending.redirect_uri != redirect_uri {
            return Err("REDIRECT_URI_MISMATCH");
        }
        if pending.challenge != pkce_challenge(verifier) {
            return Err("PKCE_MISMATCH");
        }
        pending.used = true;
        let pending = pending.clone();
        let session_id = self.next_id("session");
        self.sessions.insert(
            session_id.clone(),
            Session {
                subject: pending.subject.clone(),
                device_id: pending.device_id.clone(),
                revoked: false,
            },
        );
        let claims = Claims {
            iss: ISSUER.to_string(),
            aud: AUDIENCE.to_string(),
            sub: pending.subject,
            nonce: pending.nonce,
            iat: self.now,
            exp: self.now + 300,
            session_id: session_id.clone(),
            sync_device_id: pending.device_id,
        };
        Ok(TokenBundle {
            id_token: self.sign_claims(claims),
            session_id,
        })
    }

    fn sign_claims(&self, claims: Claims) -> String {
        let header = Header {
            alg: "EdDSA".to_string(),
            kid: self.active_kid.clone(),
            typ: "JWT".to_string(),
        };
        let encoded_header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
        let encoded_claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        let signing_input = format!("{encoded_header}.{encoded_claims}");
        let signature = self
            .signing_keys
            .get(&self.active_kid)
            .unwrap()
            .sign(signing_input.as_bytes());
        format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        )
    }

    fn public_keys(&self) -> HashMap<String, VerifyingKey> {
        self.signing_keys
            .iter()
            .map(|(kid, key)| (kid.clone(), key.verifying_key()))
            .collect()
    }

    fn native_session_is_authorized(&self, session_id: &str, subject: &str, device: &str) -> bool {
        let Some(session) = self.sessions.get(session_id) else {
            return false;
        };
        !session.revoked
            && session.subject == subject
            && session.device_id == device
            && !self.revoked_devices.contains(device)
            && !self.deleted_subjects.contains(subject)
    }

    fn logout(&mut self, session_id: &str) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.revoked = true;
        }
    }

    fn revoke_device(&mut self, device: &str) {
        self.revoked_devices.insert(device.to_string());
    }

    fn delete_account(&mut self, subject: &str) {
        self.deleted_subjects.insert(subject.to_string());
        for code in self.codes.values_mut() {
            if code.subject == subject {
                code.used = true;
            }
        }
        for session in self.sessions.values_mut() {
            if session.subject == subject {
                session.revoked = true;
            }
        }
    }

    fn rotate_signing_key(&mut self) {
        self.active_kid = "proof-key-2".to_string();
        self.signing_keys
            .insert(self.active_kid.clone(), SigningKey::from_bytes(&[8_u8; 32]));
    }

    fn retire_signing_key(&mut self, kid: &str) {
        self.signing_keys.remove(kid);
    }
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn finish_callback<'a>(
    callback: &'a Callback,
    expected_state: &str,
) -> Result<&'a str, &'static str> {
    if callback.state != expected_state {
        return Err("STATE_MISMATCH");
    }
    Ok(&callback.code)
}

fn validate_id_token(
    token: &str,
    keys: &HashMap<String, VerifyingKey>,
    now: u64,
    expected_nonce: &str,
) -> Result<Claims, &'static str> {
    let parts: Vec<_> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("TOKEN_FORMAT_INVALID");
    }
    let header: Header = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|_| "TOKEN_FORMAT_INVALID")?,
    )
    .map_err(|_| "TOKEN_FORMAT_INVALID")?;
    if header.alg != "EdDSA" || header.typ != "JWT" {
        return Err("TOKEN_ALGORITHM_INVALID");
    }
    let key = keys.get(&header.kid).ok_or("TOKEN_KEY_UNKNOWN")?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| "TOKEN_SIGNATURE_INVALID")?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| "TOKEN_SIGNATURE_INVALID")?;
    key.verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
        .map_err(|_| "TOKEN_SIGNATURE_INVALID")?;
    let claims: Claims = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| "TOKEN_FORMAT_INVALID")?,
    )
    .map_err(|_| "TOKEN_FORMAT_INVALID")?;
    if claims.iss != ISSUER {
        return Err("TOKEN_ISSUER_INVALID");
    }
    if claims.aud != AUDIENCE {
        return Err("TOKEN_AUDIENCE_INVALID");
    }
    if Uuid::parse_str(&claims.sub).is_err() {
        return Err("TOKEN_SUBJECT_INVALID");
    }
    if claims.nonce != expected_nonce {
        return Err("TOKEN_NONCE_INVALID");
    }
    if claims.iat > now || claims.exp <= now {
        return Err("TOKEN_TIME_INVALID");
    }
    Ok(claims)
}

fn complete_login(issuer: &mut IdentityIssuer) -> (Claims, TokenBundle) {
    let verifier = "fictional-pkce-verifier-with-more-than-forty-three-characters";
    let state = "fictional-state-7f36";
    let nonce = "fictional-nonce-3c91";
    let callback = issuer.authorize(
        SUBJECT,
        REDIRECT_URI,
        state,
        nonce,
        &pkce_challenge(verifier),
        DEVICE,
    );
    let code = finish_callback(&callback, state).unwrap();
    let bundle = issuer.exchange(code, verifier, REDIRECT_URI).unwrap();
    let claims =
        validate_id_token(&bundle.id_token, &issuer.public_keys(), issuer.now, nonce).unwrap();
    (claims, bundle)
}

#[test]
fn authorization_code_pkce_maps_subject_and_issues_native_session() {
    let mut issuer = IdentityIssuer::new();
    let (claims, bundle) = complete_login(&mut issuer);
    assert_eq!(claims.sub, SUBJECT);
    assert_eq!(claims.sync_device_id, DEVICE);
    assert_eq!(claims.session_id, bundle.session_id);
    assert!(issuer.native_session_is_authorized(&bundle.session_id, SUBJECT, DEVICE));
    assert!(!issuer.native_session_is_authorized(&bundle.session_id, "wrong-owner", DEVICE));
}

#[test]
fn state_pkce_redirect_expiry_and_code_replay_are_rejected() {
    let mut issuer = IdentityIssuer::new();
    let verifier = "fictional-pkce-verifier-with-more-than-forty-three-characters";
    let callback = issuer.authorize(
        SUBJECT,
        REDIRECT_URI,
        "state-a",
        "nonce-a",
        &pkce_challenge(verifier),
        DEVICE,
    );
    assert_eq!(finish_callback(&callback, "state-b"), Err("STATE_MISMATCH"));
    assert!(matches!(
        issuer.exchange(&callback.code, "wrong-verifier", REDIRECT_URI),
        Err("PKCE_MISMATCH")
    ));
    assert!(matches!(
        issuer.exchange(&callback.code, verifier, "http://127.0.0.1:1/callback"),
        Err("REDIRECT_URI_MISMATCH")
    ));
    issuer
        .exchange(&callback.code, verifier, REDIRECT_URI)
        .unwrap();
    assert!(matches!(
        issuer.exchange(&callback.code, verifier, REDIRECT_URI),
        Err("AUTH_CODE_REPLAYED")
    ));

    let expired = issuer.authorize(
        SUBJECT,
        REDIRECT_URI,
        "state-c",
        "nonce-c",
        &pkce_challenge(verifier),
        DEVICE,
    );
    issuer.now += 600;
    assert!(matches!(
        issuer.exchange(&expired.code, verifier, REDIRECT_URI),
        Err("AUTH_CODE_EXPIRED")
    ));
}

#[test]
fn issuer_audience_subject_nonce_signature_and_time_are_enforced() {
    let issuer = IdentityIssuer::new();
    let base = Claims {
        iss: ISSUER.to_string(),
        aud: AUDIENCE.to_string(),
        sub: SUBJECT.to_string(),
        nonce: "nonce-a".to_string(),
        iat: issuer.now,
        exp: issuer.now + 300,
        session_id: "session-a".to_string(),
        sync_device_id: DEVICE.to_string(),
    };
    for (mut claims, expected) in [
        (
            {
                let mut c = base.clone();
                c.iss = "https://rogue.invalid".into();
                c
            },
            "TOKEN_ISSUER_INVALID",
        ),
        (
            {
                let mut c = base.clone();
                c.aud = "other-client".into();
                c
            },
            "TOKEN_AUDIENCE_INVALID",
        ),
        (
            {
                let mut c = base.clone();
                c.sub.clear();
                c
            },
            "TOKEN_SUBJECT_INVALID",
        ),
        (
            {
                let mut c = base.clone();
                c.sub = "not-a-uuid".into();
                c
            },
            "TOKEN_SUBJECT_INVALID",
        ),
        (
            {
                let mut c = base.clone();
                c.nonce = "wrong".into();
                c
            },
            "TOKEN_NONCE_INVALID",
        ),
        (
            {
                let mut c = base.clone();
                c.exp = issuer.now;
                c
            },
            "TOKEN_TIME_INVALID",
        ),
    ] {
        let token = issuer.sign_claims(claims.clone());
        let nonce = if expected == "TOKEN_NONCE_INVALID" {
            "nonce-a"
        } else {
            &claims.nonce
        };
        assert!(
            matches!(validate_id_token(&token, &issuer.public_keys(), issuer.now, nonce), Err(error) if error == expected)
        );
        claims.sub.clear();
    }

    let mut token = issuer.sign_claims(base);
    token.push('A');
    assert!(matches!(
        validate_id_token(&token, &issuer.public_keys(), issuer.now, "nonce-a"),
        Err("TOKEN_SIGNATURE_INVALID")
    ));
}

#[test]
fn logout_device_revocation_and_account_deletion_take_effect_on_next_check() {
    let mut issuer = IdentityIssuer::new();
    let (_, first) = complete_login(&mut issuer);
    let started = Instant::now();
    issuer.logout(&first.session_id);
    assert!(!issuer.native_session_is_authorized(&first.session_id, SUBJECT, DEVICE));
    assert!(started.elapsed() < Duration::from_secs(1));

    let (_, second) = complete_login(&mut issuer);
    let started = Instant::now();
    issuer.revoke_device(DEVICE);
    assert!(!issuer.native_session_is_authorized(&second.session_id, SUBJECT, DEVICE));
    assert!(started.elapsed() < Duration::from_secs(1));

    issuer.revoked_devices.clear();
    let verifier = "fictional-pkce-verifier-with-more-than-forty-three-characters";
    let pending = issuer.authorize(
        SUBJECT,
        REDIRECT_URI,
        "state-before-delete",
        "nonce-before-delete",
        &pkce_challenge(verifier),
        DEVICE,
    );
    let (_, third) = complete_login(&mut issuer);
    let started = Instant::now();
    issuer.delete_account(SUBJECT);
    assert!(!issuer.native_session_is_authorized(&third.session_id, SUBJECT, DEVICE));
    assert!(matches!(
        issuer.exchange(&pending.code, verifier, REDIRECT_URI),
        Err("ACCOUNT_DELETED")
    ));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn signing_key_rotation_has_explicit_cache_and_retirement_behavior() {
    let mut issuer = IdentityIssuer::new();
    let stale_keys = issuer.public_keys();
    let (_, old_bundle) = complete_login(&mut issuer);
    issuer.rotate_signing_key();
    let (_, new_bundle) = complete_login(&mut issuer);

    assert!(matches!(
        validate_id_token(
            &new_bundle.id_token,
            &stale_keys,
            issuer.now,
            "fictional-nonce-3c91"
        ),
        Err("TOKEN_KEY_UNKNOWN")
    ));
    let refreshed_keys = issuer.public_keys();
    assert!(validate_id_token(
        &new_bundle.id_token,
        &refreshed_keys,
        issuer.now,
        "fictional-nonce-3c91"
    )
    .is_ok());
    assert!(validate_id_token(
        &old_bundle.id_token,
        &refreshed_keys,
        issuer.now,
        "fictional-nonce-3c91"
    )
    .is_ok());

    issuer.retire_signing_key("proof-key-1");
    let retired_keys = issuer.public_keys();
    assert!(matches!(
        validate_id_token(
            &old_bundle.id_token,
            &retired_keys,
            issuer.now,
            "fictional-nonce-3c91"
        ),
        Err("TOKEN_KEY_UNKNOWN")
    ));
}
