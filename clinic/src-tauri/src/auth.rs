//! Multi-user authentication with envelope encryption.
//!
//! One random 32-byte **master key** encrypts the patient database (SQLCipher).
//! That master key is never stored in the clear: for each user it is wrapped
//! (AES-256-GCM) under a key derived from their password (Argon2id). Any user's
//! password therefore unlocks the same data; adding a user wraps the master key
//! under their password, removing a user just drops their wrapped copy.
//!
//! `auth.json` holds only usernames, roles, salts, nonces, and wrapped keys —
//! never patient data — so it can sit unencrypted next to the encrypted DB.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;
use std::path::{Path, PathBuf};

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    Admin,
    Staff,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UserRecord {
    username: String,
    role: Role,
    salt: String,        // base64
    nonce: String,       // base64
    wrapped_key: String, // base64, master key sealed under the Argon2id KEK
    created_at: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AuthStore {
    version: u32,
    users: Vec<UserRecord>,
    #[serde(skip)]
    path: PathBuf,
}

/// The result of a successful login: who you are, and the unlocked master key.
pub struct Session {
    pub username: String,
    pub role: Role,
    pub master_key: [u8; KEY_LEN],
}

impl AuthStore {
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(e2s)?;
        let mut store: AuthStore = serde_json::from_slice(&bytes).map_err(e2s)?;
        store.path = path.to_path_buf();
        Ok(store)
    }

    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(self).map_err(e2s)?;
        std::fs::write(&self.path, json).map_err(e2s)
    }

    /// First-run: generate the master key and create the first Admin.
    pub fn initialize(
        path: &Path,
        username: &str,
        password: &str,
    ) -> Result<(Self, [u8; KEY_LEN]), String> {
        if AuthStore::exists(path) {
            return Err("Already initialized".into());
        }
        let mut master_key = [0u8; KEY_LEN];
        rand::thread_rng().fill_bytes(&mut master_key);

        let mut store = AuthStore {
            version: 1,
            users: Vec::new(),
            path: path.to_path_buf(),
        };
        store.insert_user(username, password, Role::Admin, &master_key)?;
        store.save()?;
        Ok((store, master_key))
    }

    /// Verify a password and return the unlocked master key.
    pub fn login(&self, username: &str, password: &str) -> Result<Session, String> {
        let user = self
            .find(username)
            .ok_or_else(|| "Invalid username or password".to_string())?;
        let master_key = unwrap_key(password, user).map_err(|_| {
            // Same message for unknown user and bad password — no oracle.
            "Invalid username or password".to_string()
        })?;
        Ok(Session {
            username: user.username.clone(),
            role: user.role,
            master_key,
        })
    }

    /// Admin action: add a user, wrapping the existing master key under their password.
    pub fn add_user(
        &mut self,
        master_key: &[u8; KEY_LEN],
        username: &str,
        password: &str,
        role: Role,
    ) -> Result<(), String> {
        if username.trim().is_empty() {
            return Err("Username is required".into());
        }
        if password.len() < 4 {
            return Err("Password is too short".into());
        }
        if self.find(username).is_some() {
            return Err("A user with that name already exists".into());
        }
        self.insert_user(username.trim(), password, role, master_key)?;
        self.save()
    }

    /// Admin action: remove a user. Refuses to remove the last Admin.
    pub fn remove_user(&mut self, username: &str) -> Result<(), String> {
        let target = self
            .find(username)
            .ok_or_else(|| "User not found".to_string())?;
        if target.role == Role::Admin && self.admin_count() <= 1 {
            return Err("Cannot remove the last Admin".into());
        }
        self.users.retain(|u| u.username != username);
        self.save()
    }

    /// Change a user's password by re-wrapping the master key under the new one.
    pub fn change_password(
        &mut self,
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        if new_password.len() < 4 {
            return Err("New password is too short".into());
        }
        let user = self.find(username).ok_or_else(|| "User not found".to_string())?;
        let role = user.role;
        let master_key =
            unwrap_key(old_password, user).map_err(|_| "Current password is wrong".to_string())?;
        self.users.retain(|u| u.username != username);
        self.insert_user(username, new_password, role, &master_key)?;
        self.save()
    }

    pub fn list_users(&self) -> Vec<(String, Role)> {
        self.users
            .iter()
            .map(|u| (u.username.clone(), u.role))
            .collect()
    }

    // --- internals ---

    fn insert_user(
        &mut self,
        username: &str,
        password: &str,
        role: Role,
        master_key: &[u8; KEY_LEN],
    ) -> Result<(), String> {
        let mut salt = [0u8; SALT_LEN];
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        rand::thread_rng().fill_bytes(&mut nonce);

        let kek = derive_kek(password, &salt)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&kek));
        let wrapped = cipher
            .encrypt(Nonce::from_slice(&nonce), master_key.as_slice())
            .map_err(|_| "Failed to wrap key".to_string())?;

        self.users.push(UserRecord {
            username: username.to_string(),
            role,
            salt: B64.encode(salt),
            nonce: B64.encode(nonce),
            wrapped_key: B64.encode(wrapped),
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        });
        Ok(())
    }

    fn find(&self, username: &str) -> Option<&UserRecord> {
        self.users.iter().find(|u| u.username == username)
    }

    fn admin_count(&self) -> usize {
        self.users.iter().filter(|u| u.role == Role::Admin).count()
    }
}

fn derive_kek(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], String> {
    let mut out = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|_| "Key derivation failed".to_string())?;
    Ok(out)
}

fn unwrap_key(password: &str, user: &UserRecord) -> Result<[u8; KEY_LEN], ()> {
    let salt = B64.decode(&user.salt).map_err(|_| ())?;
    let nonce = B64.decode(&user.nonce).map_err(|_| ())?;
    let wrapped = B64.decode(&user.wrapped_key).map_err(|_| ())?;
    let kek = derive_kek(password, &salt).map_err(|_| ())?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&kek));
    let plain = cipher
        .decrypt(Nonce::from_slice(&nonce), wrapped.as_slice())
        .map_err(|_| ())?; // GCM tag mismatch == wrong password
    if plain.len() != KEY_LEN {
        return Err(());
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&plain);
    Ok(key)
}

fn e2s<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("clinic_auth_test_{}_{}.json", name, std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn login_with_correct_password_unlocks_master_key() {
        let path = temp_path("login_ok");
        let (store, master) = AuthStore::initialize(&path, "admin", "secret1").unwrap();
        let session = store.login("admin", "secret1").unwrap();
        assert_eq!(session.master_key, master);
        assert_eq!(session.role, Role::Admin);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn login_with_wrong_password_fails() {
        let path = temp_path("login_bad");
        let (store, _) = AuthStore::initialize(&path, "admin", "secret1").unwrap();
        assert!(store.login("admin", "wrong").is_err());
        assert!(store.login("ghost", "secret1").is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn two_users_unlock_the_same_master_key() {
        let path = temp_path("two_users");
        let (mut store, master) = AuthStore::initialize(&path, "admin", "adminpw").unwrap();
        store
            .add_user(&master, "nurse", "nursepw", Role::Staff)
            .unwrap();
        let s1 = store.login("admin", "adminpw").unwrap();
        let s2 = store.login("nurse", "nursepw").unwrap();
        assert_eq!(s1.master_key, s2.master_key);
        assert_eq!(s2.role, Role::Staff);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn removing_a_user_revokes_only_their_access() {
        let path = temp_path("revoke");
        let (mut store, master) = AuthStore::initialize(&path, "admin", "adminpw").unwrap();
        store
            .add_user(&master, "nurse", "nursepw", Role::Staff)
            .unwrap();
        store.remove_user("nurse").unwrap();
        assert!(store.login("nurse", "nursepw").is_err());
        assert!(store.login("admin", "adminpw").is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cannot_remove_the_last_admin() {
        let path = temp_path("last_admin");
        let (mut store, _) = AuthStore::initialize(&path, "admin", "adminpw").unwrap();
        assert!(store.remove_user("admin").is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn change_password_rewraps_same_master_key() {
        let path = temp_path("chpw");
        let (mut store, master) = AuthStore::initialize(&path, "admin", "oldpw").unwrap();
        store.change_password("admin", "oldpw", "newpw").unwrap();
        assert!(store.login("admin", "oldpw").is_err());
        let session = store.login("admin", "newpw").unwrap();
        assert_eq!(session.master_key, master);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn persisted_store_reloads_and_logs_in() {
        let path = temp_path("persist");
        let (_, _) = AuthStore::initialize(&path, "admin", "secret1").unwrap();
        let reloaded = AuthStore::load(&path).unwrap();
        assert!(reloaded.login("admin", "secret1").is_ok());
        let _ = std::fs::remove_file(&path);
    }
}
