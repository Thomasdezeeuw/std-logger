// NOTE: run this benchmark with `cargo bench > /dev/null` and then open
// `target/criterion/report/index.html`.

use std::io::{self, stdout, Write};

use criterion::{criterion_group, criterion_main, Criterion};

const MSG: &[u8] = b"2020-01-06T12:00:09.514249Z [REQUEST] simple: url = `/not_found`, method = `/not_found`, status_code = 404, body_size = 9";

fn via_std_lib(c: &mut Criterion) {
    c.bench_function("via_std_lib", |b| {
        b.iter(|| {
            stdout().write(MSG).expect("write error");
        })
    });
}

fn via_libc_fd(c: &mut Criterion) {
    c.bench_function("via_libc_fd", |b| {
        b.iter(|| {
            match unsafe {
                libc::write(
                    libc::STDOUT_FILENO,
                    MSG.as_ptr() as *const libc::c_void,
                    MSG.len(),
                )
            } {
                n if n < 0 => Err(io::Error::last_os_error()),
                n => Ok(n as usize),
            }
            .expect("write error");
        })
    });
}

criterion_group!(standard_out, via_std_lib, via_libc_fd);
criterion_main!(standard_out);
