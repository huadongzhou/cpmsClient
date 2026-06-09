use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use aes::Aes128;
use base64::{engine::general_purpose, Engine as _};
use cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyInit};
use sm4::Sm4;
use uuid::Uuid;

const CLIENT_ID: &str = "0ca6eaf66cbf4f2bb1db6902c9c8d988";
const CLIENT_SECRET: &str = "8c0a5f3388154e2c959f2bd44fc2dd88";
const CLIENT_PLATFORM: &str = "pc";
const CLIENT_KEY: &str = "#iSn##s$olu.@@sm";

type Aes128EcbEncryptor = ecb::Encryptor<Aes128>;
type Sm4EcbEncryptor = ecb::Encryptor<Sm4>;

/// Generates the CPMS `access_sign` value used by HTTP requests.
pub fn sign_request(uri: &str, params: &str) -> Result<String, String> {
    let trimmed_uri = uri.trim();
    if trimmed_uri.is_empty() {
        return Err("uri 不能为空".into());
    }

    let params_map = parse_query_params(params);
    let params_sign_md5 = params_sign_md5(trimmed_uri, &params_map);
    let source = format!(
        "{}:{}:{}:{}:{}:{}",
        CLIENT_ID,
        CLIENT_SECRET,
        now_millis(),
        Uuid::new_v4(),
        CLIENT_PLATFORM,
        params_sign_md5
    );

    aes_encrypt_base64(&source, CLIENT_KEY)
}

/// Encrypts a password with the CPMS SM4-compatible algorithm.
pub fn sm4_encrypt_hex(text: &str) -> Result<String, String> {
    if text.is_empty() {
        return Ok(String::new());
    }

    let encrypted = Sm4EcbEncryptor::new_from_slice(CLIENT_KEY.as_bytes())
        .map_err(|error| error.to_string())?
        .encrypt_padded_vec_mut::<Pkcs7>(text.as_bytes());

    Ok(bytes_to_lower_hex(&encrypted))
}

/** 对参数字典和 URI 计算 MD5 签名摘要。 */
fn params_sign_md5(uri: &str, params: &HashMap<String, String>) -> String {
    let sign_source = params_string_for_signing(uri, params);
    format!("{:x}", md5::compute(sign_source.as_bytes()))
}

/** 将 URI 和参数按 key 字典序拼接为签名源字符串；空值会被跳过。 */
fn params_string_for_signing(uri: &str, params: &HashMap<String, String>) -> String {
    let mut sorted = BTreeMap::new();
    sorted.insert("requestURI".to_string(), uri.to_string());

    for (key, value) in params {
        if !value.is_empty() {
            sorted.insert(key.clone(), value.clone());
        }
    }

    sorted
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

/** 将 `k=v&k2=v2` 查询字符串解析为 HashMap；空值会被过滤。 */
fn parse_query_params(params: &str) -> HashMap<String, String> {
    params
        .split('&')
        .filter_map(|pair| {
            if pair.trim().is_empty() {
                return None;
            }

            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            if value.is_empty() {
                return None;
            }

            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

/** 使用 AES-128-ECB-PKCS7 加密文本并返回 Base64 结果。 */
fn aes_encrypt_base64(text: &str, key: &str) -> Result<String, String> {
    let encrypted = Aes128EcbEncryptor::new_from_slice(key.as_bytes())
        .map_err(|error| error.to_string())?
        .encrypt_padded_vec_mut::<Pkcs7>(text.as_bytes());

    Ok(general_purpose::STANDARD.encode(encrypted))
}

/** 将字节数组转为小写十六进制字符串。 */
fn bytes_to_lower_hex(data: &[u8]) -> String {
    let mut output = String::with_capacity(data.len() * 2);
    for byte in data {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

/** 返回当前 Unix 时间戳（毫秒）。 */
fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_signing_sorts_keys_and_skips_empty_values() {
        let params = parse_query_params("b=2&a=1&empty=&c=3");
        assert_eq!(
            params_string_for_signing("/x", &params),
            "a=1&b=2&c=3&requestURI=/x"
        );
    }

    #[test]
    fn sm4_output_is_lower_hex() {
        let encrypted = sm4_encrypt_hex("1").expect("sm4 encrypt");
        assert!(!encrypted.is_empty());
        assert!(encrypted.chars().all(|item| item.is_ascii_hexdigit()));
        assert_eq!(encrypted, encrypted.to_lowercase());
    }
}
