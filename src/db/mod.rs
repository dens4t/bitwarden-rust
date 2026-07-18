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
                security_stamp      TEXT NOT NULL DEFAULT '',
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
                deleted_date  TEXT,
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
                name  TEXT NOT NULL,
                organization_id TEXT,
                owner TEXT
            );

            CREATE TABLE IF NOT EXISTS organizations (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL DEFAULT '',
                billing_email   TEXT NOT NULL DEFAULT '',
                plan            TEXT NOT NULL DEFAULT 'TeamsStarter',
                plan_type       INTEGER NOT NULL DEFAULT 2,
                enabled         INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS organization_users (
                id              TEXT PRIMARY KEY,
                organization_id TEXT NOT NULL,
                user_id         TEXT NOT NULL,
                email           TEXT NOT NULL,
                status          INTEGER NOT NULL DEFAULT 0,
                type            INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (organization_id) REFERENCES organizations(id),
                FOREIGN KEY (user_id) REFERENCES accounts(id)
            );

            CREATE TABLE IF NOT EXISTS sends (
                id              TEXT PRIMARY KEY,
                user_id         TEXT NOT NULL,
                name            TEXT NOT NULL DEFAULT '',
                name_encrypted  INTEGER NOT NULL DEFAULT 1,
                text            TEXT,
                text_encrypted  INTEGER NOT NULL DEFAULT 1,
                file_data       TEXT,
                file_encrypted  INTEGER NOT NULL DEFAULT 1,
                max_access_count INTEGER,
                access_count    INTEGER NOT NULL DEFAULT 0,
                revision_date   TEXT NOT NULL DEFAULT (datetime('now')),
                expiration_date TEXT,
                deletion_date   TEXT,
                password        TEXT,
                disabled        INTEGER NOT NULL DEFAULT 0,
                hide_email      INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (user_id) REFERENCES accounts(id)
            );
            ",
        )?;

        // Migrate: add columns that might not exist in older DBs
        c.execute("ALTER TABLE ciphers ADD COLUMN deleted_date TEXT", []).ok();
        c.execute("ALTER TABLE accounts ADD COLUMN security_stamp TEXT NOT NULL DEFAULT ''", []).ok();
        Ok(())
    }

    // ==================== Account Operations ====================

    pub fn create_account(&self, req: &RegisterRequest) -> SqlResult<Account> {
        self.create_account_ext(
            &req.email,
            &req.name.clone().unwrap_or_default(),
            &req.resolved_hash(),
            &req.master_password_hint.clone().unwrap_or_default(),
            &req.resolved_key(),
            &req.keys.as_ref().map(|k| k.encrypted_private_key.clone()).unwrap_or_default(),
            &req.keys.as_ref().map(|k| k.public_key.clone()).unwrap_or_default(),
            req.resolved_kdf(),
            req.resolved_kdf_iterations(),
        )
    }

    pub fn create_account_ext(
        &self, email: &str, name: &str, password_hash: &str, password_hint: &str,
        key: &str, private_key: &str, public_key: &str,
        kdf: i32, kdf_iterations: i32,
    ) -> SqlResult<Account> {
        let c = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let refresh_token = crate::crypto::generate_token();
        let security_stamp = Uuid::new_v4().to_string();

        c.execute(
            "INSERT INTO accounts (id, name, email, master_password_hash, master_password_hint,
             key, private_key, public_key, refresh_token, security_stamp, kdf, kdf_iterations)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![id, name, email, password_hash, password_hint, key, private_key, public_key, refresh_token, security_stamp, kdf, kdf_iterations],
        )?;

        Ok(Account {
            id,
            name: name.to_string(),
            email: email.to_string(),
            master_password_hash: password_hash.to_string(),
            master_password_hint: password_hint.to_string(),
            key: key.to_string(),
            keys: KeyPair {
                encrypted_private_key: private_key.to_string(),
                public_key: public_key.to_string(),
            },
            refresh_token,
            two_factor_secret: String::new(),
            security_stamp,
            kdf,
            kdf_iterations,
        })
    }

    pub fn get_account_by_email(&self, email: &str) -> SqlResult<Option<Account>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, email, master_password_hash, master_password_hint, key,
             private_key, public_key, refresh_token, two_factor_secret, security_stamp, kdf, kdf_iterations
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
                security_stamp: row.get(10)?,
                kdf: row.get(11)?,
                kdf_iterations: row.get(12)?,
            })),
            None => Ok(None),
        }
    }

    pub fn get_account_by_id(&self, id: &str) -> SqlResult<Option<Account>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, email, master_password_hash, master_password_hint, key,
             private_key, public_key, refresh_token, two_factor_secret, security_stamp, kdf, kdf_iterations
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
                security_stamp: row.get(10)?,
                kdf: row.get(11)?,
                kdf_iterations: row.get(12)?,
            })),
            None => Ok(None),
        }
    }

    pub fn get_account_by_refresh_token(&self, token: &str) -> SqlResult<Option<Account>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, email, master_password_hash, master_password_hint, key,
             private_key, public_key, refresh_token, two_factor_secret, security_stamp, kdf, kdf_iterations
             FROM accounts WHERE refresh_token = ?1",
        )?;
        let mut rows = stmt.query(params![token])?;
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
                security_stamp: row.get(10)?,
                kdf: row.get(11)?,
                kdf_iterations: row.get(12)?,
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

    fn row_to_cipher(&self, data_str: String, row: &rusqlite::Row) -> SqlResult<Cipher> {
        let data: CipherData = serde_json::from_str(&data_str).unwrap_or(CipherData {
            uri: None, username: None, password: None, totp: None,
            name: None, notes: None, fields: vec![], uris: None,
        });

        let deleted_date: Option<String> = row.get(6).ok();

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
            deleted_date,
        })
    }

    pub fn list_ciphers(&self, owner: &str) -> SqlResult<Vec<Cipher>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, type, folder_id, favorite, data, revision_date, deleted_date
             FROM ciphers WHERE owner = ?1 AND deleted_date IS NULL ORDER BY id",
        )?;

        let rows = stmt.query_map(params![owner], |row| {
            let data_str: String = row.get(4)?;
            self.row_to_cipher(data_str, row)
        })?;

        let mut ciphers = Vec::new();
        for row in rows {
            ciphers.push(row?);
        }
        Ok(ciphers)
    }

    pub fn list_ciphers_including_trash(&self, owner: &str) -> SqlResult<Vec<Cipher>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, type, folder_id, favorite, data, revision_date, deleted_date
             FROM ciphers WHERE owner = ?1 ORDER BY id",
        )?;

        let rows = stmt.query_map(params![owner], |row| {
            let data_str: String = row.get(4)?;
            self.row_to_cipher(data_str, row)
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
            "SELECT id, type, folder_id, favorite, data, revision_date, deleted_date
             FROM ciphers WHERE id = ?1 AND owner = ?2",
        )?;

        let mut rows = stmt.query(params![id, owner])?;
        match rows.next()? {
            Some(row) => {
                let data_str: String = row.get(4)?;
                Ok(Some(self.row_to_cipher(data_str, row)?))
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

    pub fn soft_delete_cipher(&self, id: &str, owner: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let rows = c.execute(
            "UPDATE ciphers SET deleted_date = ?1, revision_date = ?1 WHERE id = ?2 AND owner = ?3",
            params![now, id, owner],
        )?;
        Ok(rows > 0)
    }

    pub fn restore_cipher(&self, id: &str, owner: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let rows = c.execute(
            "UPDATE ciphers SET deleted_date = NULL, revision_date = ?1 WHERE id = ?2 AND owner = ?3",
            params![now, id, owner],
        )?;
        Ok(rows > 0)
    }

    pub fn delete_cipher_permanently(&self, id: &str, owner: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let rows = c.execute(
            "DELETE FROM ciphers WHERE id = ?1 AND owner = ?2",
            params![id, owner],
        )?;
        Ok(rows > 0)
    }

    pub fn empty_trash(&self, owner: &str) -> SqlResult<usize> {
        let c = self.conn.lock().unwrap();
        let rows = c.execute(
            "DELETE FROM ciphers WHERE owner = ?1 AND deleted_date IS NOT NULL",
            params![owner],
        )?;
        Ok(rows)
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

    // ==================== Organization Operations ====================

    pub fn list_organizations(&self, user_id: &str) -> SqlResult<Vec<Organization>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT o.id, o.name, o.billing_email, o.plan, o.plan_type, o.enabled
             FROM organizations o
             JOIN organization_users ou ON ou.organization_id = o.id
             WHERE ou.user_id = ?1",
        )?;

        let rows = stmt.query_map(params![user_id], |row| {
            Ok(Organization {
                id: row.get(0)?,
                name: row.get(1)?,
                billing_email: row.get(2)?,
                plan: row.get(3)?,
                plan_type: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                object: "organization".to_string(),
            })
        })?;

        let mut orgs = Vec::new();
        for row in rows {
            orgs.push(row?);
        }
        Ok(orgs)
    }

    pub fn create_organization(&self, name: &str, billing_email: &str, user_id: &str) -> SqlResult<String> {
        let c = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();

        c.execute(
            "INSERT INTO organizations (id, name, billing_email) VALUES (?1, ?2, ?3)",
            params![id, name, billing_email],
        )?;

        // Add creator as admin (type=1)
        let ou_id = Uuid::new_v4().to_string();
        c.execute(
            "INSERT INTO organization_users (id, organization_id, user_id, email, status, type)
             VALUES (?1, ?2, ?3, ?4, 1, 1)",
            params![ou_id, id, user_id, billing_email],
        )?;

        Ok(id)
    }

    pub fn get_org_collections(&self, org_id: &str) -> SqlResult<Vec<Collection>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, organization_id, owner FROM collections WHERE organization_id = ?1",
        )?;

        let rows = stmt.query_map(params![org_id], |row| {
            Ok(Collection {
                id: row.get(0)?,
                name: row.get(1)?,
                organization_id: row.get(2)?,
                object: "collection".to_string(),
            })
        })?;

        let mut collections = Vec::new();
        for row in rows {
            collections.push(row?);
        }
        Ok(collections)
    }

    pub fn create_collection(&self, name: &str, org_id: &str, owner: &str) -> SqlResult<String> {
        let c = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        c.execute(
            "INSERT INTO collections (id, name, organization_id, owner) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, org_id, owner],
        )?;
        Ok(id)
    }

    pub fn delete_collection(&self, id: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let rows = c.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    // ==================== Send Operations ====================

    pub fn list_sends(&self, user_id: &str) -> SqlResult<Vec<Send>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, name_encrypted, text, text_encrypted, file_data, file_encrypted,
             max_access_count, access_count, revision_date, expiration_date, deletion_date,
             password, disabled, hide_email
             FROM sends WHERE user_id = ?1 ORDER BY revision_date DESC",
        )?;

        let mut sends = Vec::new();
        {
            let c = self.conn.lock().unwrap();
            let mut stmt = c.prepare(
                "SELECT id, name, name_encrypted, text, text_encrypted, file_data, file_encrypted,
                 max_access_count, access_count, revision_date, expiration_date, deletion_date,
                 password, disabled, hide_email
                 FROM sends WHERE user_id = ?1 ORDER BY revision_date DESC",
            )?;
            let rows = stmt.query_map(params![user_id], |row| {
                let file_data: Option<String> = row.get(5).unwrap_or(None);
                let file_encrypted: i32 = row.get(6).unwrap_or(0);
                Ok(Send {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    name_encrypted: row.get::<_, i32>(2).unwrap_or(1) != 0,
                    text: row.get(3)?,
                    text_encrypted: row.get::<_, i32>(4).unwrap_or(1) != 0,
                    file: file_data.map(|s| SendFile { data: s, encrypted: file_encrypted != 0 }),
                    max_access_count: row.get(7)?,
                    access_count: row.get(8)?,
                    revision_date: row.get(9)?,
                    expiration_date: row.get(10)?,
                    deletion_date: row.get(11)?,
                    password: row.get(12)?,
                    disabled: row.get::<_, i32>(13).unwrap_or(0) != 0,
                    hide_email: row.get::<_, i32>(14).unwrap_or(0) != 0,
                    object: "send".to_string(),
                })
            })?;
            for row in rows {
                sends.push(row?);
            }
        }
        Ok(sends)
    }

    pub fn create_send(&self, send: &Send, user_id: &str) -> SqlResult<String> {
        let c = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        c.execute(
            "INSERT INTO sends (id, user_id, name, name_encrypted, text, text_encrypted,
             max_access_count, access_count, revision_date, expiration_date, deletion_date,
             password, disabled, hide_email)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id, user_id, send.name, send.name_encrypted as i32,
                send.text, send.text_encrypted as i32,
                send.max_access_count, now,
                send.expiration_date, send.deletion_date,
                send.password, send.disabled as i32, send.hide_email as i32,
            ],
        )?;

        Ok(id)
    }

    pub fn get_send(&self, id: &str, user_id: &str) -> SqlResult<Option<Send>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, name_encrypted, text, text_encrypted, file_data, file_encrypted,
             max_access_count, access_count, revision_date, expiration_date, deletion_date,
             password, disabled, hide_email
             FROM sends WHERE id = ?1 AND user_id = ?2",
        )?;

        let mut rows = stmt.query(params![id, user_id])?;
        match rows.next()? {
            Some(row) => {
                let file_data: Option<String> = row.get(5)?;
                let file_encrypted: i32 = row.get(6)?;
                Ok(Some(Send {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    name_encrypted: row.get::<_, i32>(2)? != 0,
                    text: row.get(3)?,
                    text_encrypted: row.get::<_, i32>(4)? != 0,
                    file: file_data.map(|s| SendFile { data: s, encrypted: file_encrypted != 0 }),
                    max_access_count: row.get(7)?,
                    access_count: row.get(8)?,
                    revision_date: row.get(9)?,
                    expiration_date: row.get(10)?,
                    deletion_date: row.get(11)?,
                    password: row.get(12)?,
                    disabled: row.get::<_, i32>(13)? != 0,
                    hide_email: row.get::<_, i32>(14)? != 0,
                    object: "send".to_string(),
                }))
            },
            None => Ok(None),
        }
    }

    pub fn delete_send(&self, id: &str, user_id: &str) -> SqlResult<bool> {
        let c = self.conn.lock().unwrap();
        let rows = c.execute(
            "DELETE FROM sends WHERE id = ?1 AND user_id = ?2",
            params![id, user_id],
        )?;
        Ok(rows > 0)
    }

    pub fn get_user_count(&self) -> SqlResult<i64> {
        let c = self.conn.lock().unwrap();
        let count: i64 = c.query_row(
            "SELECT COUNT(*) FROM accounts", [], |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn list_all_users(&self) -> SqlResult<Vec<Account>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id, name, email, master_password_hash, master_password_hint, key,
             private_key, public_key, refresh_token, two_factor_secret, security_stamp, kdf, kdf_iterations
             FROM accounts ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Account {
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
                security_stamp: row.get(10)?,
                kdf: row.get(11)?,
                kdf_iterations: row.get(12)?,
            })
        })?;

        let mut users = Vec::new();
        for row in rows {
            users.push(row?);
        }
        Ok(users)
    }

    pub fn get_db_size(&self) -> i64 {
        let c = self.conn.lock().unwrap();
        c.query_row("SELECT page_count * page_size FROM pragma_page_count, pragma_page_size", [], |row| row.get(0)).unwrap_or(0)
    }
}
