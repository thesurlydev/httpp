use clap::Parser;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'f', long = "file", help = "Read the request from a file")]
    file: Option<String>,
    #[arg(long = "curl", help = "Convert the request to a curl command")]
    generate_curl: bool,
    #[arg(short = 's', long = "status", help = "Output the response status code")]
    include_status: bool,
    #[arg(short = 'H', long = "headers", help = "Output the response headers")]
    include_headers: bool,
    #[arg(short = 'b', long = "body", help = "Output the response body")]
    include_body: bool,
    #[arg(
        short = 'o',
        long = "output",
        help = "Specify output format (json or text)",
        default_value = "text"
    )]
    output_format: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HttpRequest {
    method: String,
    url: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    query: HashMap<String, String>,
    body: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct JsonOutput {
    status: Option<u16>,
    headers: Option<HashMap<String, String>>,
    body: Option<serde_json::Value>,
}

fn read_input(file_path: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
    match file_path {
        Some(path) => Ok(fs::read_to_string(path)?),
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            Ok(buffer)
        }
    }
}

fn generate_curl_command(request: &HttpRequest) -> String {
    let mut cmd = format!("curl -X {} ", request.method);
    for (key, value) in &request.headers {
        cmd.push_str(&format!("-H '{}:{}' ", key, value));
    }
    if !request.query.is_empty() {
        let query_string: Vec<String> = request
            .query
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        cmd.push_str(&format!("'{}?{}' ", request.url, query_string.join("&")));
    } else {
        cmd.push_str(&format!("'{}' ", request.url));
    }
    if let Some(body) = &request.body {
        cmd.push_str(&format!("-d '{}'", serde_json::to_string(body).unwrap()));
    }
    cmd
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.output_format != "json" && args.output_format != "text" {
        return Err("Invalid output format. Use 'json' or 'text'.".into());
    }

    let input = read_input(args.file)?;
    let request: HttpRequest = serde_json::from_str(&input)?;

    if args.generate_curl {
        println!("{}", generate_curl_command(&request));
        return Ok(());
    }

    let client = reqwest::Client::new();
    let mut req_builder = match request.method.as_str() {
        "GET" => client.get(&request.url),
        "POST" => client.post(&request.url),
        "PUT" => client.put(&request.url),
        "PATCH" => client.patch(&request.url),
        "DELETE" => client.delete(&request.url),
        _ => return Err("Unsupported HTTP method".into()),
    };

    // Add headers
    let mut headers = HeaderMap::new();
    for (key, value) in &request.headers {
        headers.insert(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
    }
    req_builder = req_builder.headers(headers.clone());

    // Add query parameters
    req_builder = req_builder.query(&request.query);

    // Add body for POST/PUT/PATCH requests
    if let Some(body) = &request.body {
        req_builder = req_builder.json(body);
    }

    let response = req_builder.send().await?;

    if args.output_format == "json" {
        let mut json_output = JsonOutput {
            status: None,
            headers: None,
            body: None,
        };

        if args.include_status {
            json_output.status = Some(response.status().as_u16());
        }

        if args.include_headers {
            let mut headers_map = HashMap::new();
            for (key, value) in response.headers() {
                headers_map.insert(key.to_string(), value.to_str()?.to_string());
            }
            json_output.headers = Some(headers_map);
        }

        if args.include_body {
            let body = response.text().await?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                json_output.body = Some(json);
            } else {
                json_output.body = Some(serde_json::Value::String(body));
            }
        }

        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        if args.include_status {
            println!("{}", response.status().as_u16());
        }

        if args.include_headers {
            for (key, value) in response.headers() {
                println!("{}: {}", key, value.to_str()?);
            }
        }

        if args.include_body {
            println!("");
            let body = response.text().await?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("{}", body);
            }
        }
    }

    Ok(())
}
