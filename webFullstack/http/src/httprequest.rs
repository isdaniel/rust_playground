use std::collections::HashMap;

#[derive(Debug,PartialEq)]
pub enum Method {
    GET,
    POST,
    Uninitalized
}

impl From<&str> for Method {
    fn from(s: &str) -> Self {
        match s {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => Method::Uninitalized
        }
    }
}

#[derive(Debug,PartialEq)]
pub enum Version {
    HTTP1_1,
    HTTP2,
    Uninitalized
}

impl From<&str> for Version {
    fn from(s: &str) -> Self {
        match s {
            "HTTP/1.1" => Version::HTTP1_1,
            "HTTP/2" => Version::HTTP2,
            _ => Version::Uninitalized
        }
    }
}

#[derive(Debug,PartialEq)]
pub enum Resource {
    Path(String),
    Uninitalized
}

#[derive(Debug)]
pub struct HttpRequest {
    pub method: Method,
    pub version: Version,
    pub resource: Resource,
    pub headers: HashMap<String,String>,
    pub body: String
}

impl From<String> for HttpRequest {
    fn from(req: String) -> Self {
        let mut pasred_method = Method::Uninitalized;
        let mut pasred_version = Version::Uninitalized;
        let mut pasred_resource = Resource::Uninitalized;
        let mut pasred_header = HashMap::new();
        let mut pasred_body = "";

        for line in req.lines() {
            if line.contains("HTTP") {
                let (method, resource, version) = parse_request_line(line);
                pasred_method = method;
                pasred_resource = resource;
                pasred_version = version;
            } else if line.contains(":") {
                let (key, value) = parse_header_line(line);
                pasred_header.insert(key, value);
            } else if line.is_empty() {
            } else {
                pasred_body = line;
            }
        }

        HttpRequest {
            method: pasred_method,
            version: pasred_version,
            resource: pasred_resource,
            headers: pasred_header,
            body: pasred_body.to_string()
        }
    }
}

fn parse_request_line(s:&str) -> (Method, Resource, Version) {
    let mut iter = s.split_whitespace();
    let method = Method::from(iter.next().unwrap());
    let resource = Resource::Path(iter.next().unwrap().to_string());
    let version = Version::from(iter.next().unwrap());
    (method, resource, version)
}

fn parse_header_line(s:&str) -> (String, String) {
    let mut header_items = s.split(":");
    let mut key = String::from("");
    let mut value = String::from("");
    if let Some(k) = header_items.next() {
        key = k.to_string();
    }
    if let Some(v) = header_items.next() {
        value = v.to_string();
    }
    (key, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_from_str() {
        assert_eq!(Method::from("GET"), Method::GET);
        assert_eq!(Method::from("POST"), Method::POST);
        assert_eq!(Method::from("PUT"), Method::Uninitalized);
    }

    #[test]
    fn test_version_from_str() {
        assert_eq!(Version::from("HTTP/1.1"), Version::HTTP1_1);
        assert_eq!(Version::from("HTTP/2"), Version::HTTP2);
        assert_eq!(Version::from("HTTP/3"), Version::Uninitalized);
    }

    #[test]
    fn test_parse_request_line() {
        let line = "GET / HTTP/1.1";
        let (method, resource, version) = parse_request_line(line);
        assert_eq!(method, Method::GET);
        assert_eq!(resource, Resource::Path("/".to_string()));
        assert_eq!(version, Version::HTTP1_1);
    }
    
    #[test]
    fn test_parse_header_line() {
        let line = "Host: localhost:8080";
        let (key, value) = parse_header_line(line);
        assert_eq!(key, "Host");
        assert_eq!(value, " localhost");
    }

    #[test]
    fn test_http_request_from_string() {
        let req = "GET /greeting HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\nHello, World!";
        
        let http_req = HttpRequest::from(req.to_string());
        let mut expect_headers = HashMap::new();
        expect_headers.insert("Host".to_string(), " localhost".to_string());
        expect_headers.insert("User-Agent".to_string(), " curl/7.64.1".to_string());
        expect_headers.insert("Accept".to_string(), " */*".to_string());
        assert_eq!(http_req.method, Method::GET);
        assert_eq!(http_req.version, Version::HTTP1_1);
        assert_eq!(http_req.resource, Resource::Path("/greeting".to_string()));
        assert_eq!(http_req.headers, expect_headers);
        assert_eq!(http_req.body, "Hello, World!");
    }


}