use crate::models::*;
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn: Mutex::new(conn) };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> SqlResult<()> {
        let c = self.conn.lock().unwrap();
        c.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS accounts (
                id                  TEXT PRIMARY KEY,
                name                TEXT NOT NULL DEFAULT '',
                email               TEXT UNIQUE NOT NULL,
                master_password_hash TEXT NOT NULL,
                master_password_hint TEXT NOT NULL DEFAULT '',
                key                 TEXT NOT NULL DEFAULT '',
                private_key         TEXT NOT NULL DEFAULT '',
                public_key          TEXT NOT NULL DEFAULT '',
                refresh_token       TEXT NOT NULL DEFAULT '',
                two_factor_secret   TEXT NOT NULL DEFAULT '',
                kdf                 INTEGER NOT NULL DEFAULT 0,
                kdf_iterations      INTEGER NOT NULL DEFAULT 600000,
                created_at          TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS ciphers (
                id           TEXT PRIMARY KEY,
                owner        TEXT NOT NULL,
                type         INTEGER NOT NULL DEFAULT 1,
                folder_id    TEXT,
                favorite     INTEGER NOT NULL DEFAULT 0,
                data         TEXT NOT NULL DEFAULT '{}',
                revision_date TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (owner) REFERENCES accounts(id)
            );

            CREATE TABLE IF NOT EXISTS folders (
                id            TEXT PRIMARY KEY,
                owner         TEXT NOT NULL,
                name          TEXT NOT NULL DEFAULT '',
                revision_date TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (owner) REFERENCES accounts(id)
            );

            CREATE TABLE IF NOT EXISTS collections (
                id    TEXT PRIMARY KEY,
                name  TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    // ==================== Account Operations ====================

    pub fn create_account(&self, req: &RegisterRequest) -> SqlResult<Account> {
        let c = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let refresh_token = crate::crypto::generate_token();

        c.execute(
            "INSERT INTO accounts (id, name, email, master_password_hash, master_password_hint,
             key, private_key, public_key, refresh_token, kdf, kdf_iterations)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id, req.name, req.email, req.master_password_hash,
                req.master_password_hint, req.key,
                req.keys.encrypted_private_key, req.keys.public_key,
                refresh_token, req.kdf, req.kdf_iterations,
            ],
        )?;

        Ok(Account {
            id,
            name: req.name.clone(),
            email: req.email.clone(),
            master_password_hash: req.master_password_hash.clone(),
            master_password_hint: req.master_password_hint.clone(),
            key: req.key.clone(),
            keys: KeyPair {
                encrypted_private_key: req.keys.encrypted_private_key.clone(),
                public_key: req.keys.public_key.clone(),
            },
            refresh_token,
            two_factor_secret: String::new(),
            kdf: req.kdf,
            kdf_iterations: req.kdf_iterations,
        })
    }

    pub fn get_account_by_email(&self, email: &str) -> SqlResult<Option<Account>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, email, master_password_hash, master_password_hint, key,
             private_key, public_key, refresh_token, two_factor_secret, kdf, kdf_iterations
             FROM accounts WHERE email = ?1",
        )?;

        let mut rows = stmt.query(params![email])?;
        match rows.next()? {
            Some(row) => Ok(Some(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                master_password_hash: row.get(3)?,
                master_password_hint: row.get(4)?,
                key: row.get(5)?,
                keys: KeyPair {
                    encrypted_private_key: row.get(6)?,
                    public_key: row.get(7)?,
                },
                refresh_token: row.get(8)?,
                two_factor_secret: row.get(9)?,
                kdf: row.get(10)?,
                kdf_iterations: row.get(11)?,
            })),
            None => Ok(None),
        }
    }

    pub fn get_account_by_id(&self, id: &str) -> SqlResult<Option<Account>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, email, master_password_hash, master_password_hint, key,
             private_key, public_key, refresh_token, two_factor_secret, kdf, kdf_iterations
             FROM accounts WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                master_password_hash: row.get(3)?,
                master_password_hint: row.get(4)?,
                key: row.get(5)?,
                keys: KeyPair {
                    encrypted_private_key: row.get(6)?,
                    public_key: row.get(7)?,
                },
                refresh_token: row.get(8)?,
                two_factor_secret: row.get(9)?,
                kdf: row.get(10)?,
                kdf_iterations: row.get(11)?,
            })),
            None => Ok(None),
        }
    }

    pub fn update_refresh_token(&self, id: &str, new_token: &str) -> SqlResult<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE accounts SET refresh_token = ?1 WHERE id = ?2",
            params![new_token, id],
        )?;
        Ok(())
    }

    pub fn update_keys(&self, id: &str, private_key: &str, public_key: &str) -> SqlResult<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE accounts SET private_key = ?1, public_key = ?2 WHERE id = ?3",
            params![private_key, public_key, id],
        )?;
        Ok(())
    }

    pub fn set_two_factor_secret(&self, id: &str, secret: &str) -> SqlResult<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE accounts SET two_factor_secret = ?1 WHERE id = ?2",
            params![secret, id],
        )?;
        Ok(())
    }

    pub fn get_two_factor_secret(&self, id: &str) -> SqlResult<String> {
        let c = self.conn.lock().unwrap();
        let secret: String = c.query_row(
            "SELECT two_factor_secret FROM accounts WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(secret)
    }

    pub fn disable_two_factor(&self, id: &str) -> SqlResult<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE accounts SET two_factor_secret = '' WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    // ==================== Cipher Operations ====================

    pub fn list_ciphers(&self, owner: &str) -> SqlResult<Vec<Cipher>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, type, folder_id, favorite, data, revision_date
             FROM ciphers WHERE owner = ?1 ORDER BY id",
        )?;

        let rows = stmt.query_map(params![owner], |row| {
            let data_str: String = row.get(4)?;
            let data: CipherData = serde_json::from_str(&data_str).unwrap_or(CipherData {
                uri: None, username: None, password: None, totp: None,
                name: None, notes: None, fields: vec![], uris: None,
            });

            Ok(Cipher {
                id: row.get(0)?,
                type_field: row.get(1)?,
                folder_id: row.get(2)?,
                organization_id: None,
                favorite: row.get::<_, i32>(3)? != 0,
                edit: true,
                data: Some(data.clone()),
                attachments: vec![],
                organization_use_totp: false,
                revision_date: row.get(5)?,
                object: "cipher".to_string(),
                collection_ids: vec![],
                card: None, fields: vec![], identity: None,
                login: data.uri.as_ref().or(data.username.as_ref()).map(|_| Login {
                    username: data.username.clone(),
                    password: data.password.clone(),
                    totp: data.totp.clone(),
                    uri: data.uri.clone(),
                    uris: data.uris.clone(),
                }),
                name: data.name.clone(),
                notes: data.notes.clone(),
                secure_note: data.notes.as_ref().map(|_| SecureNote { type_field: 0 }),
            })
        })?;

        let mut ciphers = Vec::new();
        for row in rows {
            ciphers.push(row?);
        }
        Ok(ciphers)
    }

    pub fn get_cipher(&self, id: &str, owner: &str) -> SqlResult<Option<Cipher>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, type, folder_id, favorite, data, revision_date
             FROM ciphers WHERE id = ?1 AND owner = ?2",
        )?;

        let mut rows = stmt.query(params![id, owner])?;
        match rows.next()? {
            Some(row) => {
                let data_str: String = row.get(4)?;
                let data: CipherData = serde_json::from_str(&data_str).unwrap_or(CipherData {
                    uri: None, username: None, password: None, totp: None,
                    name: None, notes: None, fields: vec![], uris: None,
                });

                Ok(Some(Cipher {
                    id: row.get(0)?,
                    type_field: row.get(1)?,
                    folder_id: row.get(2)?,
                    organization_id: None,
                    favorite: row.get::<_, i32>(3)? != 0,
                    edit: true,
                    data: Some(data.clone()),
                    attachments: vec![],
                    organization_use_totp: false,
                    revision_date: row.get(5)?,
                    object: "cipher".to_string(),
                    collection_ids: vec![],
                    card: None, fields: vec![], identity: None,
                    login: data.uri.as_ref().or(data.username.as_ref()).map(|_| Login {
                        username: data.username.clone(),
                        password: data.password.clone(),
                        totp: data.totp.clone(),
                        uri: data.uri.clone(),
                        uris: data.uris.clone(),
                    }),
                    name: data.name.clone(),
                    notes: data.notes.clone(),
                    secure_note: data.notes.as_ref().map(|_| SecureNote { type_field: 0 }),
                }))
            }
            None => Ok(None),
        }
    }

    pub fn create_cipher(&self, cipher: &Cipher, owner: &str) -> SqlResult<String> {
        let c = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let data_json = serde_json::to_string(&cipher.data).unwrap_or_default();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        c.execute(
            "INSERT INTO ciphers (id, owner, type, folder_id, favorite, data, revision_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, owner, cipher.type_field, cipher.folder_id,
                    cipher.favorite as i32, data_json, now],
        )?;

        Ok(id)
    }

    pub fn update_cipher(&self, id: &str, cipher: &Cipher, owner: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let data_json = serde_json::to_string(&cipher.data).unwrap_or_default();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let rows = c.execute(
            "UPDATE ciphers SET type = ?1, folder_id = ?2, favorite = ?3, data = ?4, revision_date = ?5
             WHERE id = ?6 AND owner = ?7",
            params![cipher.type_field, cipher.folder_id,
                    cipher.favorite as i32, data_json, now, id, owner],
        )?;
        Ok(rows > 0)
    }

    pub fn delete_cipher(&self, id: &str, owner: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let rows = c.execute(
            "DELETE FROM ciphers WHERE id = ?1 AND owner = ?2",
            params![id, owner],
        )?;
        Ok(rows > 0)
    }

    pub fn import_ciphers(&self, ciphers: &[Cipher], owner: &str) -> SqlResult<usize> {
        let mut count = 0;
        for cipher in ciphers {
            self.create_cipher(cipher, owner)?;
            count += 1;
        }
        Ok(count)
    }

    // ==================== Folder Operations ====================

    pub fn list_folders(&self, owner: &str) -> SqlResult<Vec<Folder>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, revision_date FROM folders WHERE owner = ?1 ORDER BY id",
        )?;

        let rows = stmt.query_map(params![owner], |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                object: "folder".to_string(),
                revision_date: row.get(2)?,
            })
        })?;

        let mut folders = Vec::new();
        for row in rows {
            folders.push(row?);
        }
        Ok(folders)
    }

    pub fn create_folder(&self, name: &str, owner: &str) -> SqlResult<String> {
        let c = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        c.execute(
            "INSERT INTO folders (id, owner, name, revision_date) VALUES (?1, ?2, ?3, ?4)",
            params![id, owner, name, now],
        )?;

        Ok(id)
    }

    pub fn update_folder(&self, id: &str, name: &str, owner: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let rows = c.execute(
            "UPDATE folders SET name = ?1, revision_date = ?2 WHERE id = ?3 AND owner = ?4",
            params![name, now, id, owner],
        )?;
        Ok(rows > 0)
    }

    pub fn delete_folder(&self, id: &str, owner: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let rows = c.execute(
            "DELETE FROM folders WHERE id = ?1 AND owner = ?2",
            params![id, owner],
        )?;
        Ok(rows > 0)
    }

    pub fn list_collections(&self, _owner: &str) -> SqlResult<Vec<serde_json::Value>> {
        Ok(vec![])
    }
}
