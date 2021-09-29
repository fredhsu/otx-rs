use tokio_postgres::NoTls;

mod otx;

async fn add_indicator_db(
    indicator: &otx::otx::Indicator,
) -> Result<u64, Box<dyn std::error::Error>> {
    let (client, connection) =
        tokio_postgres::connect("host=localhost user=fredlhsu ", NoTls).await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client
        .execute(
            "INSERT INTO indicators (indicator,indicator_type) VALUES ($1, $2)",
            &[&indicator.indicator, &indicator.indicator_type],
        )
        .await?)
}

async fn pub_indicators_nats(indicator: &otx::otx::Indicator) -> std::io::Result<()> {
    let nats_url = "10.90.226.89:32000";
    let nc = async_nats::Options::with_user_pass("ruser", "T0pS3cr3t")
        .with_name("otx-incidents")
        .connect(nats_url)
        .await?;
    let msg = format!(
        "{{\"action\" : \"published\", \"indicator\" : {}, \"count\" : 1}}",
        indicator.indicator
    );
    Ok(nc.publish("netdl.otx.indicators", msg).await?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = "6d69f7223d1c7d7f53b16270e1d7d3b9c87c3c0a5c78e00bfc0c4ac788d82e13".to_string();
    let client = otx::otx::Client::new(api)?;
    // let client = Client::new(api)?;
    let query = otx::otx::QueryParameters {
        modified_since: Some("2021-09-01T12:35:00+00:00".to_string()),
        limit: Some(10),
        types: Some(vec!["IPv4".to_string()]),
        page: None,
    };
    let resp = client.indicators_export(query).await?;
    let mut rows_modified = 0;
    for indicator in &resp.results {
        rows_modified += add_indicator_db(indicator).await?;
        if rows_modified > 0 {
            pub_indicators_nats(indicator).await?;
        }
    }
    println!("{:?}", resp);
    Ok(())
}
