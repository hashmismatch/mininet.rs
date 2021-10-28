use nom::{IResult, Parser, branch::alt, bytes::complete::tag, character::is_digit, combinator::{map, map_parser, map_res, opt}, sequence::{preceded, terminated, tuple}};
use alloc::string::{String, ToString};


#[derive(Debug, Clone)]
pub struct Url {
    pub scheme: String,
    pub authority: Authority,
    pub path: Option<String>,
    pub port: Option<u16>
}

impl Url {
    pub fn parse(url: &str) -> IResult<&[u8], Url> {

        let auth = map(map_res(nom::bytes::complete::take_till1(|c| c == ('/' as u8) || c == (':' as u8)), Authority::parse), |a| a.1);

        map(
        tuple((
            map_res(nom::character::complete::alphanumeric1, core::str::from_utf8),
            tag("://"),
            
            auth,
            opt(
                preceded(tag(":"), number_u16_complete)
            ),
            
            opt(map_res(nom::bytes::complete::take_while1(|c| true), core::str::from_utf8))
        )), |(scheme, _, authority, port, path)| {
            Url {
                scheme: scheme.to_string(),
                authority,
                path: path.map(|p| p.to_string()),
                port: port
            }
        }).parse(url.as_bytes())
    }
}

#[derive(Debug, Clone)]
pub enum Authority {
    Hostname(String),
    Ip((u8, u8, u8, u8))
}

impl Authority {
    pub fn parse(auth: &[u8]) -> IResult<&[u8], Authority> {
        alt((
            tuple((
                terminated(number_u8_complete, tag(".")),
                terminated(number_u8_complete, tag(".")),
                terminated(number_u8_complete, tag(".")),
                    number_u8_complete))
                    .map(|(a, b, c, d)| {
                        Authority::Ip((a, b, c, d))
                    }),
            map(map_res(nom::bytes::complete::take_while1(|c| true), core::str::from_utf8), |r| Authority::Hostname(r.to_string()))
        ))
        .parse(auth)
    }
}

fn number_u8_complete(input: &[u8]) -> IResult<&[u8], u8> {
    map_res(map_res(nom::bytes::complete::take_while1(is_digit), core::str::from_utf8), |s| s.parse()).parse(input)
}

fn number_u16_complete(input: &[u8]) -> IResult<&[u8], u16> {
    map_res(map_res(nom::bytes::complete::take_while1(is_digit), core::str::from_utf8), |s| s.parse()).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test_cases() {
        let urls = [
            "http://www.google.com",
            "http://127.0.0.1",
            "http://127.0.0.1:9999",
            "http://www.google.com/index.html",
            "http://foobar.com:8080/index.html"
        ];

        for url in urls {
            println!("URL: {}", url);
            let r = Url::parse(url).unwrap();
            println!("Parsed into: {:#?}", r);

            println!();
        }
    }

}