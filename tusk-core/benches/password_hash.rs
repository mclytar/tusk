use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::distributions::Alphanumeric;
use rand::Rng;
use secrecy::Secret;
use tusk_core::config::TEST_CONFIGURATION;
use tusk_core::resources::User;

fn criterion_benchmark(c: &mut Criterion) {
    let mut db_connection = TEST_CONFIGURATION.database_connect()
        .expect("database connection");

    let user = User::read_by_username(&mut db_connection, "user").expect("user");

    c.bench_function("verify against the real password", |b| b.iter(|| {
        let s: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        user.verify_password(&Secret::from(s));
    }));

    c.bench_function("verify against the fake password", |b| b.iter(|| {
        let s: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        User::fake_password_check(s);
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);