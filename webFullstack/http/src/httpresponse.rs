use std::{collections::HashMap, io::Write};

#[derive(Debug,PartialEq,Clone)]
pub struct HttpResponse<'a> {
    version: &'a str,
    status_code: &'a str,
    status_text: &'a str,
    headers: Option<HashMap<&'a str, &'a str>>,
    body: Option<String>,
}

impl<'a> Default for HttpResponse<'a> {
    fn default() -> Self {
        HttpResponse {
            version: "HTTP/1.1",
            status_code: "200",
            status_text: "OK",
            headers: None,
            body: None,
        }
    }
}

impl<'a> From<HttpResponse<'a>> for String{
    fn from(value: HttpResponse<'a>) -> Self {
        //let res = value.clone();
        let response_string = format!(
            "{} {} {}\r\n{}Content-Length:{}\r\n\r\n{}",
            value.version(),
            value.status_code(),
            value.status_text(),
            value.headers(),
            value.body().len(),
            value.body()
        );
        response_string
    }
}

impl<'a> HttpResponse<'a>{
    pub fn new(
        status_code : &'a str,
        headers: Option<HashMap<&'a str,&'a str>>,
        body:Option<String>) 
    -> HttpResponse<'a> {
        let mut res: HttpResponse<'a> = HttpResponse::default();
        if status_code != "200" {
            res.status_code = status_code;
        }
        
        res.headers = headers.or_else(|| {
            let mut h = HashMap::new();
            h.insert("Content-Type", "text/html");
            Some(h)
        });

        res.status_text = match res.status_code {
            "200" => "OK",
            "400" => "Bad Request",
            "404" => "Not Found",
            "500" => "Internal Server Error",
            _ => "Not Found!"
        };
        res.body = body;
        res
    }

    pub fn send_response(&self, write_steam: &mut impl Write) -> Result<(), std::io::Error> {
        let res = self.clone();
        let response_string:String = String::from(res);
        let _ = write!(write_steam, "{}", response_string);
        Ok(())
    }

    fn version(&self) -> &str {
        self.version
    }

    fn status_code(&self) -> &str {
        self.status_code
    }

    fn status_text(&self) -> &str {
        self.status_text
    }

    fn headers(&self) -> String {
        let map = self.headers.clone().unwrap();
        let mut header_string = String::new();
        for (key, value) in map {
            header_string.push_str(&format!("{}: {}\r\n", key, value));
        }
        header_string
    }

    pub fn body(&self) -> &str {
        match &self.body {
            Some(b) => b.as_str(),
            None => "",
        }
    }
}


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_default() {
        let res = HttpResponse::default();
        assert_eq!(res.version, "HTTP/1.1");
        assert_eq!(res.status_code, "200");
        assert_eq!(res.status_text, "OK");
        assert_eq!(res.headers, None);
        assert_eq!(res.body, None);
    }

    #[test]
    fn test_response_struct_creation_200(){
        let response_act = HttpResponse::new("200", None, Some("Hello World".to_string()));

        let response_expect = HttpResponse{
            version: "HTTP/1.1",
            status_code: "200",
            status_text: "OK",
            headers: Some({
                let mut h = HashMap::new();
                h.insert("Content-Type", "text/html");
                h
            }),
            body: Some("Hello World".to_string())
        };

        assert_eq!(response_act, response_expect);
    }
    #[test]
    fn test_response_struct_creation_404(){
        let response_act = HttpResponse::new("404", None, Some("Hello World".to_string()));

        let response_expect = HttpResponse{
            version: "HTTP/1.1",
            status_code: "404",
            status_text: "Not Found",
            headers: Some({
                let mut h = HashMap::new();
                h.insert("Content-Type", "text/html");
                h
            }),
            body: Some("Hello World".to_string())
        };

        assert_eq!(response_act, response_expect);
    }

    #[test]
    fn test_http_response_creation(){
        let response = HttpResponse::new("200", None, Some("Hello World".to_string()));
        let response_string = String::from(response);
        let expect_response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length:11\r\n\r\nHello World";
        assert_eq!(response_string, expect_response);
    }
}