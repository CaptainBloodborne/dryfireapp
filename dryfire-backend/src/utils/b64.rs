pub fn b64_encode(content: &str) -> String {
    base64_url::encode(content)
}

pub fn b64_encode_bytes(bytes: &[u8]) -> String {
    base64_url::encode(bytes)
}

pub fn b64_decode(b64u: &str) -> anyhow::Result<String> {
    let decoded_bytes = base64_url::decode(b64u)?;
    let decoded_string = String::from_utf8(decoded_bytes)?;
    Ok(decoded_string)
}

pub fn b64_decode_bytes(b64u: &str) -> anyhow::Result<Vec<u8>> {
    Ok(base64_url::decode(b64u)?)
}
