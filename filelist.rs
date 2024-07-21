use std::{fs::OpenOptions, io::{self, Write}, path::PathBuf};
use foundations::fs::*;

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let src_root: PathBuf = args.next().map(Into::into).unwrap_or_else(|| PathBuf::from("."));
    // In fact, normalize-then-sort is more reasonable, and the behavior of the `iter_path`'s `normalized`
    // is also normalize-then-sort. But since the previous behavior was sort-then-normalize, `iter_path`'s
    // `normalized` is not used, and the same behavior as before is maintained. Sorting before normalizing
    // could lead to inconsistencies in the order under different conditions! The impact of this issue
    // needs further investigation.
    let src_list = iter_path(&src_root, None, true, false).unwrap();
    let dst = args.next();
    let ffmpeg_concat = args.next().is_some();
    let mut dst_h: Box<dyn Write> = if let Some(dst_path) = dst {
        Box::new(OpenOptions::new().create_new(true).write(true).open(&dst_path).unwrap())
    } else {
        Box::new(io::stdout().lock())
    };

    macro_rules! ws {
        ($buf:expr) => {
            dst_h.write_all($buf.as_bytes()).unwrap();
        };
    }

    macro_rules! wn {
        () => {
            dst_h.write_all(b"\n").unwrap();
        };
    }

    if !ffmpeg_concat {
    ws!("NOTA Berylsoft File Fragment Hash Standard Version 2.1");
    wn!();
    wn!();
    ws!("writer=filelist@");
    ws!(env!("GIT_HASH"));
    wn!();
    wn!();
    }
    for (src_path, is_dir) in src_list {
        if !is_dir {
            // see `iter_path` call above
            let src_path = normalize(&src_path);
            let name = src_path.to_str().unwrap();
            #[cfg(windows)]
            let name = name.replace("\\", "/");
            assert!(!name.contains("\n"));
            if ffmpeg_concat {
            assert!(!name.contains("'"));
            ws!("file '");
            }
            ws!(name);
            if ffmpeg_concat {
            ws!("'");
            }
            wn!();
        }
    }
}
