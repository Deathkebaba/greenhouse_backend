use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use greenhouse_core::auth_service_dto::register_admin::RegisterAdminRequestDto;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::{self, Postgres};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use tokio::task::JoinHandle;
#[allow(dead_code)]
pub const TEST_USERNAME: &str = "testuser";
#[allow(dead_code)]
pub const TEST_PASSWORD: &str = "testpassword";
pub const AUTH_SECRET: &str = "testpassword";

pub struct TestContext {
    auth_postgres_container: Option<ContainerAsync<Postgres>>,
    data_storage_postgres_container: Option<ContainerAsync<Postgres>>,
    device_postgres_container: Option<ContainerAsync<Postgres>>,
    scripting_postgres_container: Option<ContainerAsync<Postgres>>,
    auth_service: Option<JoinHandle<Result<(), std::io::Error>>>,
    data_storage_service: Option<JoinHandle<Result<(), std::io::Error>>>,
    scripting_service: Option<JoinHandle<Result<(), std::io::Error>>>,
    device_service: Option<JoinHandle<Result<(), std::io::Error>>>,
    web_api: Option<JoinHandle<Result<(), std::io::Error>>>,
    scripting_api: Option<JoinHandle<Result<(), std::io::Error>>>,
}

impl TestContext {
    pub fn new() -> Self {
        Self {
            auth_postgres_container: None,
            data_storage_postgres_container: None,
            device_postgres_container: None,
            scripting_postgres_container: None,
            auth_service: None,
            data_storage_service: None,
            scripting_service: None,
            device_service: None,
            web_api: None,
            scripting_api: None,
        }
    }

    pub async fn start_all_services(&mut self) {
        if self.data_storage_postgres_container.is_none() {
            self.data_storage_postgres_container = Some(start_data_storage_postgres().await);
        }
        let data_port = self
            .data_storage_postgres_container
            .as_ref()
            .unwrap()
            .get_host_port_ipv4(5432)
            .await
            .unwrap();

        if self.device_postgres_container.is_none() {
            self.device_postgres_container = Some(start_device_postgres().await);
        }
        let device_port = self
            .device_postgres_container
            .as_ref()
            .unwrap()
            .get_host_port_ipv4(5432)
            .await
            .unwrap();

        if self.scripting_postgres_container.is_none() {
            self.scripting_postgres_container = Some(start_scripting_postgres().await);
        }
        let scripting_port = self
            .scripting_postgres_container
            .as_ref()
            .unwrap()
            .get_host_port_ipv4(5432)
            .await
            .unwrap();

        if self.auth_postgres_container.is_none() {
            self.auth_postgres_container = Some(start_auth_postgres().await);
        }
        let auth_port = self
            .auth_postgres_container
            .as_ref()
            .unwrap()
            .get_host_port_ipv4(5432)
            .await
            .unwrap();

        if self.auth_service.is_none() {
            self.auth_service = Some(
                start_auth_service(format!(
                    "postgres://postgres:postgres@localhost:{auth_port}/auth"
                ))
                .await,
            );
        }

        if self.device_service.is_none() {
            self.device_service = Some(
                start_device_service(format!(
                    "postgres://postgres:postgres@localhost:{device_port}/device"
                ))
                .await,
            );
        }
        if self.scripting_service.is_none() {
            self.scripting_service = Some(
                start_scripting_service(format!(
                    "postgres://postgres:postgres@localhost:{scripting_port}/scripting"
                ))
                .await,
            );
        }
        if self.data_storage_service.is_none() {
            self.data_storage_service = Some(
                start_data_storage_service(format!(
                    "postgres://postgres:postgres@localhost:{data_port}/data"
                ))
                .await,
            );
        }
        if self.scripting_service.is_none() {
            self.scripting_service = None;
        }
        if self.web_api.is_none() {
            self.web_api = Some(start_web_api().await);
        }
        if self.scripting_api.is_none() {
            self.scripting_api = Some(start_scripting_api().await);
        }
    }

    pub async fn stop(&self) {
        if let Some(container) = &self.auth_postgres_container {
            container.stop().await.unwrap();
        }
        if let Some(container) = &self.data_storage_postgres_container {
            container.stop().await.unwrap();
        }
        if let Some(container) = &self.device_postgres_container {
            container.stop().await.unwrap();
        }
        if let Some(auth_service) = &self.auth_service {
            auth_service.abort();
        }
        if let Some(data_storage_service) = &self.data_storage_service {
            data_storage_service.abort();
        }
        if let Some(device_service) = &self.device_service {
            device_service.abort();
        }
        if let Some(web_api) = &self.web_api {
            web_api.abort();
        }
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

async fn start_auth_postgres() -> ContainerAsync<Postgres> {
    postgres::Postgres::default()
        .with_db_name("auth")
        .with_tag("latest")
        .start()
        .await
        .unwrap()
}

async fn start_data_storage_postgres() -> ContainerAsync<Postgres> {
    postgres::Postgres::default()
        .with_db_name("data")
        .with_tag("latest")
        .start()
        .await
        .unwrap()
}

async fn start_device_postgres() -> ContainerAsync<Postgres> {
    postgres::Postgres::default()
        .with_db_name("device")
        .with_tag("latest")
        .start()
        .await
        .unwrap()
}

async fn start_scripting_postgres() -> ContainerAsync<Postgres> {
    postgres::Postgres::default()
        .with_db_name("scripting")
        .with_tag("latest")
        .start()
        .await
        .unwrap()
}

async fn start_auth_service(db_url: String) -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
    let auth_service_config = auth_service::Config {
        database_url: db_url,
        service_port: 3001,
        sentry_url: String::new(),
        jwt_secret: AUTH_SECRET.to_string(),
        environment: String::from("test"),
    };

    let auth_pool = auth_service::Pool::builder()
        .build(AsyncDieselConnectionManager::new(
            auth_service_config.database_url.clone(),
        ))
        .await
        .unwrap();
    let auth_service_app = auth_service::app(auth_service_config.clone(), auth_pool.clone());

    let url = format!("0.0.0.0:{}", auth_service_config.service_port);
    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    tokio::spawn(async move { axum::serve(listener, auth_service_app).await })
}

async fn start_device_service(
    db_url: String,
) -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
    let device_service_config = device_service::Config {
        database_url: db_url,
        service_port: 3003,
        sentry_url: String::new(),
        scripting_service: String::from("http://localhost:3004"),
        scripting_api: String::from("http://localhost:3100"),
        environment: String::from("test"),
        prometheus_url: String::from("http://localhost:9090"),
    };

    let device_pool = device_service::Pool::builder()
        .build(AsyncDieselConnectionManager::new(
            device_service_config.database_url.clone(),
        ))
        .await
        .unwrap();
    let device_service_app =
        device_service::test_app(device_service_config.clone(), device_pool.clone());

    let url = format!("0.0.0.0:{}", device_service_config.service_port);
    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    tokio::spawn(async move { axum::serve(listener, device_service_app).await })
}

async fn start_data_storage_service(
    db_url: String,
) -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
    let data_storage_service_config = data_storage_service::Config {
        database_url: db_url,
        service_port: 3002,
        sentry_url: String::new(),
        environment: String::from("test"),
    };

    let data_storage_pool = data_storage_service::Pool::builder()
        .build(AsyncDieselConnectionManager::new(
            data_storage_service_config.database_url.clone(),
        ))
        .await
        .unwrap();
    let data_storage_service_app = data_storage_service::app(
        data_storage_service_config.clone(),
        data_storage_pool.clone(),
    );

    let url = format!("0.0.0.0:{}", data_storage_service_config.service_port);
    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    tokio::spawn(async move { axum::serve(listener, data_storage_service_app).await })
}

async fn start_scripting_service(
    db_url: String,
) -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
    let scripting_service_config = scripting_service::Config {
        database_url: db_url,
        service_port: 3004,
        sentry_url: String::new(),
        scripting_api: String::from("http://localhost:3100"),
        environment: String::from("test"),
    };

    let scripting_pool = scripting_service::Pool::builder()
        .build(AsyncDieselConnectionManager::new(
            scripting_service_config.database_url.clone(),
        ))
        .await
        .unwrap();
    let scripting_service_app =
        scripting_service::app(scripting_service_config.clone(), scripting_pool.clone());

    let url = format!("0.0.0.0:{}", scripting_service_config.service_port);
    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    tokio::spawn(async move { axum::serve(listener, scripting_service_app).await })
}
async fn start_web_api() -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
    let web_api_config = web_api::Config {
        api_port: 3000,
        service_addresses: web_api::ServiceAddresses {
            auth_service: String::from("http://localhost:3001"),
            data_storage_service: String::from("http://localhost:3002"),
            device_service: String::from("http://localhost:3003"),
        },
        sentry_url: String::new(),
        environment: String::from("test"),
    };

    let api_app = web_api::app(web_api_config.clone());
    let url = format!("0.0.0.0:{}", web_api_config.api_port);
    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    tokio::spawn(async move { axum::serve(listener, api_app).await })
}

async fn start_scripting_api() -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
    let scripting_api_config = scripting_api::Config {
        api_port: 3100,
        service_addresses: scripting_api::ServiceAddresses {
            data_storage_service: String::from("http://localhost:3002"),
            scripting_service: String::from("http://localhost:3004"),
        },
        sentry_url: String::new(),
        environment: String::from("test"),
    };

    let api_app = scripting_api::app(scripting_api_config.clone());
    let url = format!("0.0.0.0:{}", scripting_api_config.api_port);
    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    tokio::spawn(async move { axum::serve(listener, api_app).await })
}
#[allow(dead_code)]
pub async fn admin_login() -> String {
    register_admin().await;

    api_login().await
}
#[allow(dead_code)]
pub async fn register_admin() {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3001/registerAdmin")
        .json(&RegisterAdminRequestDto {
            username: TEST_USERNAME.to_string(),
            password: TEST_PASSWORD.to_string(),
        })
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Failed to register admin");
}
#[allow(dead_code)]
pub async fn api_login() -> String {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/api/login")
        .json(&greenhouse_core::auth_service_dto::login::LoginRequestDto {
            username: TEST_USERNAME.to_string(),
            password: TEST_PASSWORD.to_string(),
        })
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Failed api login");
    let user_token: greenhouse_core::auth_service_dto::login::LoginResponseDto =
        response.json().await.unwrap();
    user_token.token
}

#[allow(dead_code)]
pub async fn scripting_login() -> String {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3004/token")
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success(), "Failed scripting login");
    let user_token: String = response.text().await.unwrap();
    user_token
}
