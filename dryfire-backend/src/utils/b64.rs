pub fn b64_encode(content: &str) -> String {
    base64_url::encode(content)
}

pub fn b64_decode(b64u: &str) -> anyhow::Result<String> {
    // let decoded_string = base64_url::decode(b64u)
    //     .and_then(|r| String::from_utf8(r))

    let decoded_bytes = base64_url::decode(b64u)?;
    let decoded_string = String::from_utf8(decoded_bytes)?;

    Ok(decoded_string)
}