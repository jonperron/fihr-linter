const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE64URL_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64_encode_with(input: &[u8], alphabet: &[u8]) -> String {
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };
        out.push(alphabet[(b0 >> 2) & 0x3f] as char);
        out.push(alphabet[((b0 << 4) | (b1 >> 4)) & 0x3f] as char);
        if chunk.len() > 1 {
            out.push(alphabet[((b1 << 2) | (b2 >> 6)) & 0x3f] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(alphabet[b2 & 0x3f] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub(super) fn base64_encode(input: &[u8]) -> String {
    base64_encode_with(input, BASE64_CHARS)
}

pub(super) fn base64_encode_url(input: &[u8]) -> String {
    base64_encode_with(input, BASE64URL_CHARS).replace('=', "")
}

fn base64_decode_with(input: &[u8], alphabet: &[u8]) -> Vec<u8> {
    let lookup: Vec<i8> = {
        let mut table = vec![-1i8; 256];
        for (i, &c) in alphabet.iter().enumerate() {
            table[c as usize] = i as i8;
        }
        table
    };
    let filtered: Vec<u8> = input
        .iter()
        .filter(|&&c| c != b'=' && lookup[c as usize] >= 0)
        .copied()
        .collect();
    let mut out = Vec::new();
    for chunk in filtered.chunks(4) {
        let get = |i: usize| {
            if i < chunk.len() {
                lookup[chunk[i] as usize].max(0) as u8
            } else {
                0
            }
        };
        let b0 = get(0);
        let b1 = get(1);
        let b2 = get(2);
        let b3 = get(3);
        out.push((b0 << 2) | (b1 >> 4));
        if chunk.len() > 2 {
            out.push((b1 << 4) | (b2 >> 2));
        }
        if chunk.len() > 3 {
            out.push((b2 << 6) | b3);
        }
    }
    out
}

pub(super) fn base64_decode(input: &[u8]) -> Vec<u8> {
    base64_decode_with(input, BASE64_CHARS)
}

pub(super) fn base64_decode_url(input: &[u8]) -> Vec<u8> {
    base64_decode_with(input, BASE64URL_CHARS)
}
