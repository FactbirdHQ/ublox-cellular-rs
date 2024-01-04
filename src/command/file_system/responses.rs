//! Responses for File system Commands
use atat::atat_derive::AtatResp;
use atat::heapless_bytes::Bytes;
use heapless::String;

/// 22.4 Read file +URDFILE
#[derive(Debug, PartialEq, Eq, AtatResp)]
pub struct ReadFileResponse {
    #[at_arg(position = 0)]
    pub filename: String<248>,
    #[at_arg(position = 1)]
    pub size: usize,
    // TODO: Streaming data?
    #[at_arg(position = 2)]
    pub data: Bytes<{ 1024 + 2 }>,
}

/// 22.5 Partial read file +URDBLOCK
#[derive(Clone, Debug, PartialEq, Eq, AtatResp)]
pub struct ReadBlockResponse {
    #[at_arg(position = 0)]
    pub filename: String<248>,
    #[at_arg(position = 1)]
    pub size: usize,
    #[at_arg(position = 2)]
    pub data: Bytes<{ 512 + 2 }>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn deserialize_read_file_response() {
        let resp = b"+URDFILE: \"response.txt\",655,\"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 74\r\nConnection: close\r\nDate: Wed, 14 Jul 2021 11:50:30 GMT\r\nx-amzn-RequestId: 021e3877-1e6d-447d-996e-4bc89087bdc5\r\nx-amz-apigw-id: CdVdFFJhjoEFc0w=\r\nX-Amzn-Trace-Id: Root=1-60eecf86-227f4f986747c3113846cd63;Sampled=1\r\nVia: 1.1 32e3b86ae254a231182567c0124af893.cloudfront.net (CloudFront), 1.1 2afacc6ad96dbba3f0b477cd95f16459.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Error from cloudfront\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Amz-Cf-Id: FELxJa2hgelObvyEP16HS4yEK-emXa1NiMsRXl-rmarzg309KeD34g==\r\n\r\n{\"uuid\": \"what\"}\"";

        let exp = ReadFileResponse {
            filename: String::try_from("response.txt").unwrap(),
            size: 655,
            data: Bytes::from_slice(b"\"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 74\r\nConnection: close\r\nDate: Wed, 14 Jul 2021 11:50:30 GMT\r\nx-amzn-RequestId: 021e3877-1e6d-447d-996e-4bc89087bdc5\r\nx-amz-apigw-id: CdVdFFJhjoEFc0w=\r\nX-Amzn-Trace-Id: Root=1-60eecf86-227f4f986747c3113846cd63;Sampled=1\r\nVia: 1.1 32e3b86ae254a231182567c0124af893.cloudfront.net (CloudFront), 1.1 2afacc6ad96dbba3f0b477cd95f16459.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Error from cloudfront\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Amz-Cf-Id: FELxJa2hgelObvyEP16HS4yEK-emXa1NiMsRXl-rmarzg309KeD34g==\r\n\r\n{\"uuid\": \"what\"}\"").unwrap(),
        };

        assert_eq!(atat::serde_at::from_slice(resp), Ok(exp));
    }

    #[test]
    #[ignore]
    fn deserialize_partial_block_response() {
        let resp = b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: 25\r\nConnection: close\r\nDate: Mon, 19 Jul 2021 07:23:35 GMT\r\nx-amzn-RequestId: 4a50eb56-5c1a-4388-9a2f-a1966ba9c8a2\r\nx-amz-apigw-id: CtNCvF1SDoEF2dw=\r\nX-Amzn-Trace-Id: Root=1-60f52877-6f5b63ac154d314436832848;Sampled=1\r\nVia: 1.1 58b222ebbb6cc6c8c8c9a46127ae3a3e.cloudfront.net (CloudFront), 1.1 6fa33d47af6f4da7007689083cfe9b9c.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Error from cloudfront\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Amz-\"";

        let exp = ReadBlockResponse {
            filename: String::try_from("response.txt").unwrap(),
            size: 512,
            data: Bytes::from_slice(b"\"HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: 25\r\nConnection: close\r\nDate: Mon, 19 Jul 2021 07:23:35 GMT\r\nx-amzn-RequestId: 4a50eb56-5c1a-4388-9a2f-a1966ba9c8a2\r\nx-amz-apigw-id: CtNCvF1SDoEF2dw=\r\nX-Amzn-Trace-Id: Root=1-60f52877-6f5b63ac154d314436832848;Sampled=1\r\nVia: 1.1 58b222ebbb6cc6c8c8c9a46127ae3a3e.cloudfront.net (CloudFront), 1.1 6fa33d47af6f4da7007689083cfe9b9c.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Error from cloudfront\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Amz-\"").unwrap(),
        };

        assert_eq!(atat::serde_at::from_slice(resp), Ok(exp));
    }

    #[test]
    #[ignore]
    fn deserialize_partial_block_response_ok() {
        let resp = b"+URDBLOCK: \"response.txt\",512,\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"";

        let exp = ReadBlockResponse {
            filename: String::try_from("response.txt").unwrap(),
            size: 512,
            data: Bytes::from_slice(b"\"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2553\r\nConnection: close\r\nVary: Accept-Encoding\r\nDate: Mon, 19 Jul 2021 07:47:39 GMT\r\nx-amzn-RequestId: 436ba5b8-2aad-4089-a4fd-1b1c38773c87\r\nx-amz-apigw-id: CtQkMFE_DoEFUzg=\r\nX-Amzn-Trace-Id: Root=1-60f52e1a-0a05343260f3ba3331eea9d6;Sampled=1\r\nVia: 1.1 f99b5b46e77cfe9c3413f99dc8a4088c.cloudfront.net (CloudFront), 1.1 2f194b62c8c43859cbf5af8e53a8d2a7.cloudfront.net (CloudFront)\r\nX-Amz-Cf-Pop: FRA2-C2\r\nX-Cache: Miss from cloudfront\r\nX-Amz-Cf-Pop\"").unwrap(),
        };

        assert_eq!(atat::serde_at::from_slice(resp), Ok(exp));
    }

    #[test]
    fn deserialize_certificate() {
        let resp = b"+URDBLOCK: \"response.txt\",512,\": FRA2-C2\r\nX-Amz-Cf-Id: _5ZSzv-MrL1yMkdklMqbtggquF-NEe6lO36pw9cYsKJVEITyIdrbqQ==\r\n\r\n{\"Data\":{\"certificate_pem\":\"-----BEGIN CERTIFICATE-----\nMIIDWjCCAkKgAwIBAgIVANeQUG3TupBxD8FLSz+AAqxU7rU0MA0GCSqGSIb3DQEB\nCwUAME0xSzBJBgNVBAsMQkFtYXpvbiBXZWIgU2VydmljZXMgTz1BbWF6b24uY29t\nIEluYy4gTD1TZWF0dGxlIFNUPVdhc2hpbmd0b24gQz1VUzAeFw0yMTA3MjIwOTA2\nMTlaFw00OTEyMzEyMzU5NTlaMB4xHDAaBgNVBAMME0FXUyBJb1QgQ2VydGlmaWNh\ndGUwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQCqCFWHSRH35wSjP0SR\nQijGwEfWArPaqr33S80y9D\"";

        let exp = ReadBlockResponse {
            filename: String::try_from("response.txt").unwrap(),
            size: 512,
            data: Bytes::from_slice(b"\": FRA2-C2\r\nX-Amz-Cf-Id: _5ZSzv-MrL1yMkdklMqbtggquF-NEe6lO36pw9cYsKJVEITyIdrbqQ==\r\n\r\n{\"Data\":{\"certificate_pem\":\"-----BEGIN CERTIFICATE-----\nMIIDWjCCAkKgAwIBAgIVANeQUG3TupBxD8FLSz+AAqxU7rU0MA0GCSqGSIb3DQEB\nCwUAME0xSzBJBgNVBAsMQkFtYXpvbiBXZWIgU2VydmljZXMgTz1BbWF6b24uY29t\nIEluYy4gTD1TZWF0dGxlIFNUPVdhc2hpbmd0b24gQz1VUzAeFw0yMTA3MjIwOTA2\nMTlaFw00OTEyMzEyMzU5NTlaMB4xHDAaBgNVBAMME0FXUyBJb1QgQ2VydGlmaWNh\ndGUwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQCqCFWHSRH35wSjP0SR\nQijGwEfWArPaqr33S80y9D\"").unwrap(),
        };

        assert_eq!(atat::serde_at::from_slice(resp), Ok(exp));
    }
}
