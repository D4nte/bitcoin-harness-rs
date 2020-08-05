use std::time::Duration;
use testcontainers::{clients, images::coblox_bitcoincore::BitcoinCore, Container, Docker};
use bitcoincore_rpc::{Client, Auth, RpcApi};
use url::Url;

pub use bitcoincore_rpc::bitcoin;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Bitcoind<'c> {
    pub container: Container<'c, clients::Cli, BitcoinCore>,
    pub node_url: Url,
    pub wallet_name: String,
}

impl<'c> Bitcoind<'c> {
    /// Starts a new regtest bitcoind container
    pub fn new(client: &'c clients::Cli, tag: &str) -> Result<Self> {
        let container = client.run(BitcoinCore::default().with_tag(tag));
        let port = container.get_host_port(18443);

        let auth = container.image().auth();
        let url = format!(
            "http://{}:{}@localhost:{}",
            &auth.username,
            &auth.password,
            port.unwrap()
        );
        let url = Url::parse(&url)?;

        let wallet_name = String::from("testwallet");

        Ok(Self {
            container,
            node_url: url,
            wallet_name,
        })
    }

    /// Create a test wallet, generate enough block to fund it and activate segwit.
    /// Generate enough blocks to make the passed `spendable_quantity` spendable.
    /// Spawn a tokio thread to mine a new block every second.
    pub async fn init(&self, spendable_quantity: u64) -> Result<()> {
        let bitcoind_client = Client::new(self.node_url.clone(), Auth::None)?;

        bitcoind_client
            .create_wallet(&self.wallet_name, None, None, None, None)
            .await?;

        let reward_address = bitcoind_client
            .get_new_address(&self.wallet_name, None, None)
            .await?;

        bitcoind_client
            .generate_to_address(101 + spendable_quantity, reward_address.clone(), None)
            .await?;
        let _ = tokio::spawn(mine(bitcoind_client, reward_address));

        Ok(())
    }

    /// Send Bitcoin to the specified address, limited to the spendable bitcoin quantity.
    pub async fn mint(
        &self,
        address: bitcoin::Address,
        amount: bitcoin::Amount,
    ) -> anyhow::Result<()> {
        let bitcoind_client = bitcoin::Client::new(self.node_url.clone());

        bitcoind_client
            .send_to_address(&self.wallet_name, address.clone(), amount)
            .await?;

        Ok(())
    }

    pub fn container_id(&self) -> &str {
        self._container.id()
    }
}

async fn mine(
    bitcoind_client: bitcoin::Client,
    reward_address: bitcoin::Address,
) -> anyhow::Result<()> {
    loop {
        tokio::time::delay_for(Duration::from_secs(1)).await;
        bitcoind_client
            .generate_to_address(1, reward_address.clone(), None)
            .await?;
    }
}

pub struct Error;