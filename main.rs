#[inline]
fn usize_u64(n: usize) -> u64 {
    n.try_into().expect("FATAL: usize length to u64 error")
}

use std::{fs::{self, OpenOptions}, io::{self, Read, Write}, path::{Path, PathBuf}};

fn normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek() {
        let buf = PathBuf::from(c.as_os_str());
        components.next();
        buf
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }

    ret
}

pub fn iter_path_core(
    current_path: &Path,
    depth: Option<u64>,
    currect_depth: u64,
    list: &mut Vec<(PathBuf, bool)>,
) -> io::Result<()> {
    for entry in fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();
        let is_dir = entry.file_type()?.is_dir();
        if let Some(depth) = depth {
            if currect_depth >= depth {
                continue;
            }
        }
        #[cfg(windows)]
        if entry.file_name() == "System Volume Information" || entry.file_name() == "$RECYCLE.BIN" {
            continue;
        }
        if is_dir {
            iter_path_core(&path, depth, currect_depth + 1, list)?;
        }
        list.push((path, is_dir));
    }
    Ok(())
}

pub fn iter_path(path: &Path, depth: Option<u64>) -> io::Result<Vec<(PathBuf, bool)>> {
    let mut list = Vec::new();
    let is_dir = fs::metadata(path)?.file_type().is_dir();
    if is_dir {
        iter_path_core(path, depth, 0, &mut list)?;
    } else {
        list.push((path.to_owned(), is_dir));
    }
    list.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(list)
}

use cshake::{CShakeCustom, cshake_customs, Absorb, Squeeze, Reset};

const HEADER: &str = include_str!("header");

cshake_customs! {
    DEFAULT_CUSTOM -> "BerylsoftFragHashV1"
    DEFAULT_SUM_CUSTOM -> "BerylsoftFragHashSumV1"
}

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let frag = args.next().unwrap();
    let frag: usize = frag.to_str().unwrap().parse().unwrap();
    let frag = frag * 1048576;
    let src_root: PathBuf = args.next().map(Into::into).unwrap_or_else(|| PathBuf::from("."));
    let src_list = iter_path(&src_root, None).unwrap();
    let dst = args.next();
    let mut dst_h: Box<dyn Write> = if let Some(dst_path) = dst {
        Box::new(OpenOptions::new().create_new(true).write(true).open(&dst_path).unwrap())
    } else {
        Box::new(io::stdout().lock())
    };
    let mut info_h = io::stderr().lock();
    let mut buf = vec![0u8; frag];
    let mut len_buf = itoa::Buffer::new();
    let mut hash_buf = [0; 64];
    let mut hash_str_buf = [0; 128];
    let mut ctx = DEFAULT_CUSTOM.create();
    let mut sum_ctx = DEFAULT_SUM_CUSTOM.create();

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

    ws!(HEADER);
    wl!(frag);
    wn!();
    for (src_path, is_dir) in src_list {
        if !is_dir {
            wn!();
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
                    break;
                }
            }
        }
    }
}
