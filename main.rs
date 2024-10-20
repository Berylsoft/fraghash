#![deny(unused_results)]

#[inline]
fn usize_u64(n: usize) -> u64 {
    n.try_into().expect("FATAL: usize length to u64 error")
}

use std::{fs::OpenOptions, io::{self, Read, Write}, path::PathBuf};
use foundations::fs::*;
use cshake::{CShakeCustom, cshake_customs, Absorb, Squeeze, Reset};

cshake_customs! {
    BerylsoftFragHashV1 -> "BerylsoftFragHashV1"
    BerylsoftFragHashSumV1 -> "BerylsoftFragHashSumV1"
}

fn alg_name(outsize: usize) -> &'static str {
    match outsize {
        32 => "cshake256_256",
        64 => "cshake256_512",
        _ => unreachable!(),
    }
}

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let frag = args.next().unwrap();
    let frag: usize = frag.to_str().unwrap().parse().unwrap();
    let frag = frag * 1048576;
    let outsize = args.next().unwrap();
    let outsize: usize = outsize.to_str().unwrap().parse().unwrap();
    let alg_name_str = alg_name(outsize);
    let src_root: PathBuf = args.next().map(Into::into).unwrap_or_else(|| PathBuf::from("."));
    // In fact, normalize-then-sort is more reasonable, and the behavior of the `iter_path`'s `normalized`
    // is also normalize-then-sort. But since the previous behavior was sort-then-normalize, `iter_path`'s
    // `normalized` is not used, and the same behavior as before is maintained. Sorting before normalizing
    // could lead to inconsistencies in the order under different conditions! The impact of this issue
    // needs further investigation.
    let src_list = iter_path(&src_root, None, true, false).unwrap();
    let dst = args.next();
    let mut dst_h: Box<dyn Write> = if let Some(dst_path) = dst {
        Box::new(OpenOptions::new().create_new(true).write(true).open(&dst_path).unwrap())
    } else {
        Box::new(io::stdout().lock())
    };
    let mut info_h = io::stderr().lock();
    let mut buf = vec![0; frag];
    let mut len_buf = itoa::Buffer::new();
    let mut hash_buf = vec![0; outsize];
    let mut hash_str_buf = vec![0; outsize * 2];
    let mut ctx = BerylsoftFragHashV1.create();
    let mut sum_ctx = BerylsoftFragHashSumV1.create();

    macro_rules! w {
        ($buf:expr) => {
            dst_h.write_all(&$buf).unwrap();
        };
    }

    macro_rules! ws {
        ($buf:expr) => {
            dst_h.write_all($buf.as_bytes()).unwrap();
        };
    }

    macro_rules! wl {
        ($buf:expr) => {
            dst_h.write_all(len_buf.format($buf).as_bytes()).unwrap();
        };
    }

    macro_rules! wn {
        () => {
            dst_h.write_all(b"\n").unwrap();
        };
    }

    macro_rules! iws {
        ($buf:expr) => {
            info_h.write_all($buf.as_bytes()).unwrap();
        };
    }

    macro_rules! iwl {
        ($buf:expr) => {
            info_h.write_all(len_buf.format($buf).as_bytes()).unwrap();
        };
    }

    macro_rules! iwn {
        () => {
            info_h.write_all(b"\n").unwrap();
        };
    }

    ws!("Berylsoft File Fragment Hash Standard Version 2.1");
    wn!();
    wn!();
    ws!("writer=fraghash@");
    ws!(env!("GIT_HASH"));
    wn!();
    ws!("alg=");
    ws!(alg_name_str);
    wn!();
    ws!("custom=");
    w!(BerylsoftFragHashV1.custom_string());
    wn!();
    ws!("sum_alg=");
    ws!(alg_name_str);
    wn!();
    ws!("sum_custom=");
    w!(BerylsoftFragHashSumV1.custom_string());
    wn!();
    ws!("frag=");
    wl!(frag);
    wn!();
    for (src_path, is_dir) in src_list {
        if !is_dir {
            wn!();
            // see `iter_path` call above
            let src_path = normalize(&src_path);
            let name = src_path.to_str().unwrap();
            #[cfg(windows)]
            let name = name.replace("\\", "/");
            ws!("name(");
            wl!(name.len());
            ws!(")=");
            ws!(name);
            wn!();
            iws!(name);
            iwn!();
            let mut src_f = OpenOptions::new().read(true).open(&src_path).unwrap();
            let len = src_f.metadata().unwrap().len();
            ws!("size=");
            let mut progress = 0;
            wl!(len);
            wn!();
            let mut block_count: u64 = 0;
            loop {
                let read_len = src_f.read(&mut buf).unwrap();
                if read_len != 0 {
                    // buf == buf[..read_len] when buf_len == read_len
                    let buf = &mut buf[..read_len];
                    ctx.absorb(buf);
                    sum_ctx.absorb(buf);
                    ctx.squeeze(&mut hash_buf);
                    hex::encode_to_slice(&hash_buf, &mut hash_str_buf).unwrap();
                    w!(hash_str_buf);
                    ws!(" ");
                    wl!(block_count);
                    wn!();
                    ctx.reset();
                    progress += usize_u64(read_len);
                    block_count += 1;
                    iwl!(progress);
                    iws!(" ");
                } else {
                    // must be EOF beacuse buf_len != 0
                    assert_eq!(progress, len);
                    iwn!();
                    sum_ctx.squeeze(&mut hash_buf);
                    hex::encode_to_slice(&hash_buf, &mut hash_str_buf).unwrap();
                    w!(hash_str_buf);
                    ws!(" SUM");
                    wn!();
                    // The original designed semantics of "sum hash" was `hash(current_file_content)`,
                    // but since the sum ctx was not cleared after every file finished when it was
                    // implemented, and such an implementation had already been put into production
                    // before the bug was found, the semantics had to be changed to
                    // `hash(all_previous_files_content | current_file_content)`. Not only does it
                    // make sense on its own, though, but it makes sense of fragment-separated
                    // hash with only one fragment.
                    break;
                }
            }
        }
    }
}
