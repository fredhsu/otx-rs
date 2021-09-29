pub mod otx {
    use reqwest::header::HeaderMap;
    use reqwest::Url;
    use serde::{Deserialize, Serialize};

    static OTX_DEFAULT_EXCHANGE: &str = "https://otx.alienvault.com";

    pub struct QueryParameters {
        pub limit: Option<u8>,
        pub page: Option<String>,
        pub types: Option<Vec<String>>,
        pub modified_since: Option<String>,
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
    pub struct Pulse {
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
        pub results: Vec<Indicator>,
        pub count: u32,
        pub previous: Option<String>,
        pub next: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Indicator {
        pub id: i32,
        pub indicator: String,
        #[serde(rename = "type")]
        pub indicator_type: String,
        pub title: Option<String>,
        pub description: Option<String>,
        pub content: String,
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
}
