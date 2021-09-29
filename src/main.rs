use reqwest::header::HeaderMap;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;

mod otx;

static OTX_DEFAULT_EXCHANGE: &str = "https://otx.alienvault.com";

pub struct QueryParameters {
    limit: Option<u8>,
    page: Option<String>,
    types: Option<Vec<String>>,
    modified_since: Option<String>,
}

impl QueryParameters {
    pub fn to_vec(&self) -> Vec<(String, String)> {
        let mut vec = Vec::new();
        match &self.limit {
            Some(l) => vec.push(("limit".to_string(), l.to_string())),
            None => (),
        };
        match &self.page {
            Some(p) => vec.push(("page".to_string(), p.to_owned())),
            None => (),
        };
        match &self.types {
            Some(types) => {
                let types_values = types.join(",");
                vec.push(("types".to_string(), types_values));
            }
            None => (),
        };
        match &self.modified_since {
            Some(ms) => vec.push(("modified_since".to_string(), ms.to_owned())),
            None => (),
        };
        vec
    }
    pub fn build_url(&self, url: &str) -> Result<String, url::ParseError> {
        let params = self.to_vec();
        let url = Url::parse_with_params(url, params)?;
        Ok(url.as_str().to_string())
    }
}

#[allow(dead_code)]
struct Pulse {
    id: String,
    name: String,
    description: String,
    author_name: String,
    modified: String,
    created: String,
    revision: u8,
    tlp: String,
    public: u8,
    adversary: String,
    //indicators: Vec<Int>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Export {
    results: Vec<Indicator>,
    count: u32,
    previous: Option<String>,
    next: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Indicator {
    id: i32,
    indicator: String,
    #[serde(rename = "type")]
    indicator_type: String,
    title: Option<String>,
    description: Option<String>,
    content: String,
}

pub struct Client {
    // api_key: String,
    // pub base_url: String,
    pub client: reqwest::Client,
}

impl Client {
    pub fn new(api_key: String) -> Result<Client, reqwest::Error> {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());
        headers.insert("X-OTX-API-KEY", api_key.parse().unwrap());
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Client {
            // api_key,
            client,
        })
    }
    pub async fn indicators_export(
        &self,
        params: QueryParameters,
    ) -> Result<Export, reqwest::Error> {
        let base_url = format!("{}/api/v1/indicators/export", OTX_DEFAULT_EXCHANGE);
        let url = params.build_url(&base_url).unwrap();
        let resp = self.client.get(url).send().await?.json::<Export>().await?;
        Ok(resp)
    }
}

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
