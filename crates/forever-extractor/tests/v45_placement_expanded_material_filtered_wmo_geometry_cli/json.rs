// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub enum JsonValue {
    Object(BTreeMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    String(String),
    Number(String),
    Scalar,
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn whitespace(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
    }

    fn byte(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn take(&mut self, expected: u8) -> Result<(), &'static str> {
        if self.byte() == Some(expected) {
            self.at += 1;
            Ok(())
        } else {
            Err("invalid JSON token")
        }
    }

    fn hex_quad(&mut self) -> Result<u16, &'static str> {
        let digits = self
            .bytes
            .get(self.at..self.at + 4)
            .ok_or("short JSON unicode escape")?;
        let mut value = 0_u16;
        for digit in digits {
            value = value.checked_mul(16).ok_or("invalid JSON unicode escape")?
                + match digit {
                    b'0'..=b'9' => u16::from(digit - b'0'),
                    b'a'..=b'f' => u16::from(digit - b'a') + 10,
                    b'A'..=b'F' => u16::from(digit - b'A') + 10,
                    _ => return Err("invalid JSON unicode escape"),
                };
        }
        self.at += 4;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, &'static str> {
        self.take(b'"')?;
        let mut value = String::new();
        loop {
            match self.byte().ok_or("unterminated JSON string")? {
                b'"' => {
                    self.at += 1;
                    return Ok(value);
                }
                b'\\' => {
                    self.at += 1;
                    match self.byte().ok_or("unterminated JSON escape")? {
                        b'"' => value.push('"'),
                        b'\\' => value.push('\\'),
                        b'/' => value.push('/'),
                        b'b' => value.push('\u{0008}'),
                        b'f' => value.push('\u{000c}'),
                        b'n' => value.push('\n'),
                        b'r' => value.push('\r'),
                        b't' => value.push('\t'),
                        b'u' => {
                            self.at += 1;
                            let first = self.hex_quad()?;
                            let scalar = if (0xd800..=0xdbff).contains(&first) {
                                if self.bytes.get(self.at..self.at + 2) != Some(b"\\u") {
                                    return Err("unpaired JSON surrogate");
                                }
                                self.at += 2;
                                let second = self.hex_quad()?;
                                if !(0xdc00..=0xdfff).contains(&second) {
                                    return Err("unpaired JSON surrogate");
                                }
                                0x10000
                                    + ((u32::from(first) - 0xd800) << 10)
                                    + (u32::from(second) - 0xdc00)
                            } else if (0xdc00..=0xdfff).contains(&first) {
                                return Err("unpaired JSON surrogate");
                            } else {
                                u32::from(first)
                            };
                            value.push(char::from_u32(scalar).ok_or("invalid JSON scalar")?);
                            continue;
                        }
                        _ => return Err("invalid JSON escape"),
                    }
                    self.at += 1;
                }
                0..=0x1f => return Err("control byte in JSON string"),
                _ => {
                    let tail = std::str::from_utf8(&self.bytes[self.at..])
                        .map_err(|_| "invalid UTF-8 JSON string")?;
                    let character = tail.chars().next().ok_or("invalid UTF-8 JSON string")?;
                    self.at += character.len_utf8();
                    value.push(character);
                }
            }
        }
    }

    fn value(&mut self) -> Result<JsonValue, &'static str> {
        self.whitespace();
        match self.byte().ok_or("missing JSON value")? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(JsonValue::String(self.string()?)),
            b't' => self.literal(b"true"),
            b'f' => self.literal(b"false"),
            b'n' => self.literal(b"null"),
            b'-' | b'0'..=b'9' => Ok(JsonValue::Number(self.number()?)),
            _ => Err("invalid JSON value"),
        }
    }

    fn object(&mut self) -> Result<JsonValue, &'static str> {
        self.take(b'{')?;
        self.whitespace();
        let mut fields = BTreeMap::new();
        if self.byte() == Some(b'}') {
            self.at += 1;
            return Ok(JsonValue::Object(fields));
        }
        loop {
            self.whitespace();
            let key = self.string()?;
            if fields.contains_key(&key) {
                return Err("duplicate JSON object key");
            }
            self.whitespace();
            self.take(b':')?;
            let value = self.value()?;
            fields.insert(key, value);
            self.whitespace();
            match self.byte() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(JsonValue::Object(fields));
                }
                _ => return Err("unterminated JSON object"),
            }
        }
    }

    fn array(&mut self) -> Result<JsonValue, &'static str> {
        self.take(b'[')?;
        self.whitespace();
        let mut values = Vec::new();
        if self.byte() == Some(b']') {
            self.at += 1;
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.whitespace();
            match self.byte() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(JsonValue::Array(values));
                }
                _ => return Err("unterminated JSON array"),
            }
        }
    }

    fn literal(&mut self, expected: &[u8]) -> Result<JsonValue, &'static str> {
        if self.bytes.get(self.at..self.at + expected.len()) == Some(expected) {
            self.at += expected.len();
            Ok(JsonValue::Scalar)
        } else {
            Err("invalid JSON literal")
        }
    }

    fn number(&mut self) -> Result<String, &'static str> {
        let start = self.at;
        if self.byte() == Some(b'-') {
            self.at += 1;
        }
        match self.byte() {
            Some(b'0') => {
                self.at += 1;
                if self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
                    return Err("leading zero in JSON number");
                }
            }
            Some(b'1'..=b'9') => {
                self.at += 1;
                while self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
                    self.at += 1;
                }
            }
            _ => return Err("invalid JSON number"),
        }
        if self.byte() == Some(b'.') {
            self.at += 1;
            let fraction = self.at;
            while self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
                self.at += 1;
            }
            if self.at == fraction {
                return Err("invalid JSON fraction");
            }
        }
        if self.byte().is_some_and(|byte| matches!(byte, b'e' | b'E')) {
            self.at += 1;
            if self.byte().is_some_and(|byte| matches!(byte, b'+' | b'-')) {
                self.at += 1;
            }
            let exponent = self.at;
            while self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
                self.at += 1;
            }
            if self.at == exponent {
                return Err("invalid JSON exponent");
            }
        }
        std::str::from_utf8(&self.bytes[start..self.at])
            .map(str::to_owned)
            .map_err(|_| "invalid JSON number")
    }
}

pub fn parse(input: &str) -> Result<JsonValue, &'static str> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        at: 0,
    };
    let value = parser.value()?;
    parser.whitespace();
    if parser.at != parser.bytes.len() {
        return Err("trailing JSON data");
    }
    Ok(value)
}
