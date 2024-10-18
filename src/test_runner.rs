use anyhow::{Context, Result};
use clap::Parser;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: PathBuf,
}

#[derive(serde::Deserialize, Debug)]
struct TestSpec {
    name: String,
    base_url: String,
    tests: Vec<Test>,
    variables: std::collections::HashMap<String, String>,
}

#[derive(serde::Deserialize, Debug)]
struct Test {
    name: String,
    request: Request,
    expected_response: ExpectedResponse,
    predicates: Vec<Predicate>,
}

#[derive(serde::Deserialize, Debug)]
struct Request {
    method: String,
    url: String,
    headers: Option<std::collections::HashMap<String, String>>,
    query: Option<std::collections::HashMap<String, String>>,
    body: Option<Value>,
}

#[derive(serde::Deserialize, Debug)]
struct ExpectedResponse {
    status_code: u16,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<Value>,
}

#[derive(serde::Deserialize, Debug)]
struct Predicate {
    description: String,
    rule: String,
    behavior: String,
}

async fn run_test(
    test: &Test,
    base_url: &str,
    variables: &std::collections::HashMap<String, String>,
) -> Result<()> {
    println!("Running test: {}", test.name);

    let client = reqwest::Client::new();
    let url = format!("{}{}", base_url, test.request.url);
    let method = match test.request.method.as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        _ => anyhow::bail!("Unsupported HTTP method: {}", test.request.method),
    };

    let mut req_builder = client.request(method, &url);

    // Add headers
    if let Some(headers) = &test.request.headers {
        let mut header_map = HeaderMap::new();
        for (key, value) in headers {
            let resolved_value = resolve_variables(value, variables);
            header_map.insert(
                HeaderName::from_str(key).context("Failed to parse header name")?,
                HeaderValue::from_str(&resolved_value).context("Failed to parse header value")?,
            );
        }
        req_builder = req_builder.headers(header_map);
    }

    // Add query parameters
    if let Some(query) = &test.request.query {
        req_builder = req_builder.query(&query);
    }

    // Add body
    if let Some(body) = &test.request.body {
        req_builder = req_builder.json(body);
    }

    let response = req_builder.send().await.context("Failed to send request")?;

    // Validate response
    let status_matches = response.status().as_u16() == test.expected_response.status_code;
    println!("Status code match: {}", status_matches);

    if let Some(expected_headers) = &test.expected_response.headers {
        for (key, value) in expected_headers {
            if let Some(actual_value) = response.headers().get(key) {
                let matches = actual_value.to_str().unwrap_or("") == value;
                println!("Header '{}' match: {}", key, matches);
            } else {
                println!("Header '{}' is missing", key);
            }
        }
    }

    if let Some(expected_body) = &test.expected_response.body {
        let actual_body: Value = response
            .json()
            .await
            .context("Failed to parse response body")?;
        let body_matches = expected_body == &actual_body;
        println!("Body match: {}", body_matches);
    }

    // TODO: Implement predicate evaluation

    Ok(())
}

fn resolve_variables(value: &str, variables: &std::collections::HashMap<String, String>) -> String {
    let mut result = value.to_string();
    for (key, var_value) in variables {
        result = result.replace(&format!("{{{{{}}}}}", key), var_value);
    }
    result
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let spec_content = fs::read_to_string(&args.file).context("Could not read spec file")?;

    let spec: TestSpec =
        serde_json::from_str(&spec_content).context("Could not parse spec file")?;

    println!("Running test suite: {}", spec.name);

    for test in &spec.tests {
        run_test(test, &spec.base_url, &spec.variables).await?;
    }

    println!("All tests completed.");

    Ok(())
}
