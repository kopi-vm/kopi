# ADR-005: Web API Mocking Strategy for Unit Testing

## Status
Proposed

## Context
Kopi relies on the Foojay.io API to fetch JDK metadata and download information. To ensure robust unit testing without depending on external services, we need a strategy for mocking Web API calls. The project uses `attohttpc` as its HTTP client, which is a synchronous HTTP client that doesn't have built-in mocking capabilities.

## Decision

### Mocking Approach
We will adopt a dual-strategy approach for handling Web API mocking:

1. **Trait Abstraction Pattern** for unit tests - Create an abstraction layer around HTTP operations
2. **Mock Server** for integration tests - Use actual HTTP servers that respond with predefined data

### Architecture

#### HTTP Client Trait
```rust
// src/http/client.rs
use std::time::Duration;
use crate::error::Result;

#[cfg_attr(test, mockall::automock)]
pub trait HttpClient: Send + Sync {
    fn get(&self, url: &str) -> Result<HttpResponse>;
    fn get_with_timeout(&self, url: &str, timeout: Duration) -> Result<HttpResponse>;
    fn download(&self, url: &str, path: &Path) -> Result<()>;
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| KopiError::InvalidResponse(e.to_string()))
    }
    
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| KopiError::Json(e))
    }
}
```

#### Production Implementation
```rust
// src/http/attohttpc_client.rs
pub struct AttohttpcClient {
    base_timeout: Duration,
}

impl HttpClient for AttohttpcClient {
    fn get(&self, url: &str) -> Result<HttpResponse> {
        let response = attohttpc::get(url)
            .timeout(self.base_timeout)
            .send()
            .map_err(|e| KopiError::Http(e))?;
        
        Ok(HttpResponse {
            status: response.status().as_u16(),
            headers: extract_headers(&response),
            body: response.bytes()
                .map_err(|e| KopiError::Http(e))?,
        })
    }
    
    fn download(&self, url: &str, path: &Path) -> Result<()> {
        let mut response = attohttpc::get(url)
            .send()
            .map_err(|e| KopiError::Http(e))?;
        
        let mut file = std::fs::File::create(path)
            .map_err(|e| KopiError::Io(e))?;
        
        response.write_to(&mut file)
            .map_err(|e| KopiError::Download(e.to_string()))?;
        
        Ok(())
    }
}
```

#### Foojay API Client
```rust
// src/foojay/client.rs
pub struct FoojayClient<C: HttpClient> {
    http_client: C,
    base_url: String,
}

impl<C: HttpClient> FoojayClient<C> {
    pub fn new(http_client: C) -> Self {
        Self {
            http_client,
            base_url: "https://api.foojay.io".to_string(),
        }
    }
    
    pub fn list_distributions(&self) -> Result<Vec<Distribution>> {
        let url = format!("{}/disco/v3.0/distributions", self.base_url);
        let response = self.http_client.get(&url)?;
        
        if response.status != 200 {
            return Err(KopiError::ApiError(response.status, response.text()?));
        }
        
        let data: FoojayResponse<Distribution> = response.json()?;
        Ok(data.result)
    }
}
```

### Testing Strategy

#### Unit Tests with Mocks
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    
    #[test]
    fn test_list_distributions() {
        let mut mock_client = MockHttpClient::new();
        
        mock_client
            .expect_get()
            .with(eq("https://api.foojay.io/disco/v3.0/distributions"))
            .times(1)
            .returning(|_| {
                Ok(HttpResponse {
                    status: 200,
                    headers: HashMap::new(),
                    body: br#"{
                        "result": [
                            {
                                "id": "temurin",
                                "name": "Eclipse Temurin",
                                "vendor": "Eclipse Foundation"
                            }
                        ]
                    }"#.to_vec(),
                })
            });
        
        let foojay = FoojayClient::new(mock_client);
        let distributions = foojay.list_distributions().unwrap();
        
        assert_eq!(distributions.len(), 1);
        assert_eq!(distributions[0].id, "temurin");
    }
    
    #[test]
    fn test_api_error_handling() {
        let mut mock_client = MockHttpClient::new();
        
        mock_client
            .expect_get()
            .returning(|_| {
                Ok(HttpResponse {
                    status: 503,
                    headers: HashMap::new(),
                    body: b"Service Unavailable".to_vec(),
                })
            });
        
        let foojay = FoojayClient::new(mock_client);
        let result = foojay.list_distributions();
        
        assert!(matches!(result, Err(KopiError::ApiError(503, _))));
    }
}
```

#### Integration Tests with Mock Server
```rust
// tests/foojay_integration.rs
use mockito::Server;

#[test]
fn test_full_jdk_installation_flow() {
    let mut server = Server::new();
    
    // Mock the distributions endpoint
    let distributions_mock = server
        .mock("GET", "/disco/v3.0/distributions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/distributions.json"))
        .create();
    
    // Mock the packages endpoint
    let packages_mock = server
        .mock("GET", "/disco/v3.0/packages")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_body(include_str!("fixtures/packages.json"))
        .create();
    
    // Create client with mock server URL
    let http_client = AttohttpcClient::new();
    let mut foojay = FoojayClient::new(http_client);
    foojay.base_url = server.url();
    
    // Test the flow
    let distributions = foojay.list_distributions().unwrap();
    assert!(!distributions.is_empty());
    
    let packages = foojay.get_packages("temurin", "21").unwrap();
    assert!(!packages.is_empty());
    
    // Verify mocks were called
    distributions_mock.assert();
    packages_mock.assert();
}
```

### Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
# Existing dependencies...

[dev-dependencies]
mockall = "0.12"
mockito = "1.4"
```

### Testing Guidelines

1. **Unit Tests** should use the `MockHttpClient` to test business logic in isolation
2. **Integration Tests** should use `mockito` to test the full HTTP flow
3. **Test Fixtures** should be stored in `tests/fixtures/` for realistic API responses
4. **Error Cases** must be thoroughly tested, including:
   - Network errors
   - Invalid JSON responses
   - HTTP error status codes
   - Timeout scenarios

## Consequences

### Positive
- Clear separation between HTTP operations and business logic
- Fast, deterministic unit tests without network dependencies
- Easy to test error scenarios and edge cases
- Flexibility to change HTTP clients in the future
- Type-safe mocking with compile-time guarantees

### Negative
- Additional abstraction layer adds complexity
- Need to maintain mock implementations
- Slightly more boilerplate code for tests
- Two different testing approaches to learn (unit vs integration)

### Neutral
- Team members need to understand when to use mocks vs mock servers
- Test fixtures need to be kept up-to-date with API changes
- Mock setup can be verbose for complex scenarios

## Implementation Plan

1. Create the `HttpClient` trait in `src/http/client.rs`
2. Implement `AttohttpcClient` wrapper
3. Add `mockall` to dev-dependencies
4. Refactor existing code to use the trait
5. Add comprehensive unit tests with mocks
6. Add `mockito` for integration tests
7. Create test fixtures from actual API responses
8. Document testing patterns in the project README

## References
- [mockall documentation](https://docs.rs/mockall/latest/mockall/)
- [mockito documentation](https://docs.rs/mockito/latest/mockito/)
- [Rust testing best practices](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [HTTP mocking in Rust comparison](https://blog.logrocket.com/testing-http-requests-rust/)