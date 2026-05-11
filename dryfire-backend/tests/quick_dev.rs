#![allow(unused)]

use anyhow::Result;

#[tokio::test]
async fn quick_dev() -> Result<()> {
    let hc = httpc_test::new_client("http://localhost:8000")?;

    hc.do_get("/health").await?.print().await?;

    println!("->> Hi");

    Ok(())

}