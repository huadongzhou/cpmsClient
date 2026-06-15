//! token 静态加密：用每安装随机生成的本地密钥（受限权限文件）对落盘 token 做 AES 加密，
//! 避免凭据以明文保存。注意：密钥与密文同盘，属「本地密钥静态加密」（防误读/误同步），
//! 强度不及 OS keychain；后续可替换为系统凭据库。

use std::fs;
use std::path::Path;

use aes::Aes128;
use base64::{engine::general_purpose, Engine as _};
use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyInit};
use uuid::Uuid;

const KEY_FILE: &str = ".token-key";
const ENC_PREFIX: &str = "enc:v1:";

type Aes128EcbEnc = ecb::Encryptor<Aes128>;
type Aes128EcbDec = ecb::Decryptor<Aes128>;

/// 加密 token，返回带 `enc:v1:` 前缀的 base64 串；失败则原样返回明文。
pub(crate) fn encrypt(dir: &Path, plaintext: &str) -> String {
    if plaintext.is_empty() {
        return plaintext.to_string();
    }

    let Some(key) = load_or_create_key(dir) else {
        return plaintext.to_string();
    };

    match Aes128EcbEnc::new_from_slice(&key) {
        Ok(cipher) => {
            let encrypted = cipher.encrypt_padded_vec_mut::<Pkcs7>(plaintext.as_bytes());
            format!("{ENC_PREFIX}{}", general_purpose::STANDARD.encode(encrypted))
        }
        Err(_) => plaintext.to_string(),
    }
}

/// 解密落盘 token；非加密前缀视为旧明文原样返回；解密失败返回 None（按未登录处理）。
pub(crate) fn decrypt(dir: &Path, value: &str) -> Option<String> {
    let Some(body) = value.strip_prefix(ENC_PREFIX) else {
        return Some(value.to_string());
    };

    let key = load_or_create_key(dir)?;
    let bytes = general_purpose::STANDARD.decode(body).ok()?;
    let cipher = Aes128EcbDec::new_from_slice(&key).ok()?;
    let decrypted = cipher.decrypt_padded_vec_mut::<Pkcs7>(&bytes).ok()?;
    String::from_utf8(decrypted).ok()
}

fn load_or_create_key(dir: &Path) -> Option<[u8; 16]> {
    let path = dir.join(KEY_FILE);

    if let Ok(raw) = fs::read(&path) {
        if raw.len() >= 16 {
            let mut key = [0u8; 16];
            key.copy_from_slice(&raw[..16]);
            return Some(key);
        }
    }

    let key = *Uuid::new_v4().as_bytes();
    fs::create_dir_all(dir).ok()?;
    fs::write(&path, key).ok()?;
    restrict_permissions(&path);
    Some(key)
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {
    // Windows 下密钥文件位于用户专属 AppData 目录，依赖目录 ACL 保护。
}
