#[inline]
fn usize_u64(n: usize) -> u64 {
    n.try_into().expect("FATAL: usize length to u64 error")
}

use std::{fs::OpenOptions, io::{Read, Write}};
use cshake::{CShakeCustom, cshake_customs, Absorb, Squeeze, Reset};

const HEADER: &str = include_str!("header");

cshake_customs! {
    DEFAULT_CUSTOM -> "BerylsoftFragHashV1"
    DEFAULT_SUM_CUSTOM -> "BerylsoftFragHashSumV1"
}

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let src = args.next().expect("src not provided");
    let dst = args.next().expect("dst not provided");
    let mut src_f = OpenOptions::new().read(true).open(&src).unwrap();
    let mut dst_f = OpenOptions::new().create_new(true).write(true).open(&dst).unwrap();
    let mut buf = vec![0u8; 16777216];
    let mut ctx = DEFAULT_CUSTOM.create();
    let mut sum_ctx = DEFAULT_SUM_CUSTOM.create();
    let len = src_f.metadata().unwrap().len();
    let mut progress = 0;
    dst_f.write_all(HEADER.as_bytes()).unwrap();
    let mut len_str = len.to_string();
    len_str.push_str("\n\n");
    dst_f.write_all(len_str.as_bytes()).unwrap();
    let mut hash_buf = [0; 64];
    let mut hash_str_buf = [0; 129];
    hash_str_buf[128] = b'\n';
    loop {
        let read_len = src_f.read(&mut buf).unwrap();
        if read_len != 0 {
            // buf == buf[..read_len] when buf_len == read_len
            let buf = &mut buf[..read_len];
            ctx.absorb(buf);
            sum_ctx.absorb(buf);
            ctx.squeeze(&mut hash_buf);
            hex::encode_to_slice(&hash_buf, &mut hash_str_buf[..128]).unwrap();
            dst_f.write_all(&hash_str_buf).unwrap();
            ctx.reset();
            progress += usize_u64(read_len);
        } else {
            // must be EOF beacuse buf_len != 0
            assert_eq!(progress, len);
            dst_f.write_all(b"\n").unwrap();
            sum_ctx.squeeze(&mut hash_buf);
            hex::encode_to_slice(&hash_buf, &mut hash_str_buf[..128]).unwrap();
            dst_f.write_all(&hash_str_buf).unwrap();
            break;
        }
    }
}
