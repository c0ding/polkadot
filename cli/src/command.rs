// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use log::info;
use service::{IdentifyVariant, self};
use sc_executor::NativeExecutionDispatch;
use sc_cli::{SubstrateCli, Result};
use crate::cli::{Cli, Subcommand};

impl SubstrateCli for Cli {
	fn impl_name() -> &'static str { "Parity Polkadot" }

	fn impl_version() -> &'static str { env!("SUBSTRATE_CLI_IMPL_VERSION") }

	fn description() -> &'static str { env!("CARGO_PKG_DESCRIPTION") }

	fn author() -> &'static str { env!("CARGO_PKG_AUTHORS") }

	fn support_url() -> &'static str { "https://github.com/paritytech/polkadot/issues/new" }

	fn copyright_start_year() -> i32 { 2017 }

	fn executable_name() -> &'static str { "polkadot" }

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"polkadot-dev" | "dev" => Box::new(service::chain_spec::polkadot_development_config()),
			"polkadot-local" => Box::new(service::chain_spec::polkadot_local_testnet_config()),
			"polkadot-staging" => Box::new(service::chain_spec::polkadot_staging_testnet_config()),
			"kusama-dev" => Box::new(service::chain_spec::kusama_development_config()),
			"kusama-local" => Box::new(service::chain_spec::kusama_local_testnet_config()),
			"kusama-staging" => Box::new(service::chain_spec::kusama_staging_testnet_config()),
			"westend" => Box::new(service::chain_spec::westend_config()?),
			"kusama" | "" => Box::new(service::chain_spec::kusama_config()?),
			"westend-dev" => Box::new(service::chain_spec::westend_development_config()),
			"westend-local" => Box::new(service::chain_spec::westend_local_testnet_config()),
			"westend-staging" => Box::new(service::chain_spec::westend_staging_testnet_config()),
			path if self.run.force_kusama => {
				Box::new(service::KusamaChainSpec::from_json_file(std::path::PathBuf::from(path))?)
			},
			path if self.run.force_westend => {
				Box::new(service::WestendChainSpec::from_json_file(std::path::PathBuf::from(path))?)
			},
			path => Box::new(service::PolkadotChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		})
	}
}

/// Parses polkadot specific CLI arguments and run the service.
pub fn run() -> Result<()> {
	sc_cli::reset_signal_pipe_handler()?;

	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runtime = cli.create_runner(&cli.run.base)?;
			let config = runtime.config();
			let authority_discovery_enabled = cli.run.authority_discovery_enabled;
			let grandpa_pause = if cli.run.grandpa_pause.is_empty() {
				None
			} else {
				Some((cli.run.grandpa_pause[0], cli.run.grandpa_pause[1]))
			};

			if config.chain_spec.is_kusama() {
				info!("----------------------------");
				info!("This chain is not in any way");
				info!("      endorsed by the       ");
				info!("     KUSAMA FOUNDATION      ");
				info!("----------------------------");

				runtime.run_node(
					|config| {
						service::kusama_new_light(config)
					},
					|config| {
						service::kusama_new_full(
							config,
							None,
							None,
							authority_discovery_enabled,
							6000,
							grandpa_pause
						).map(|(s, _, _)| s)
					},
					service::KusamaExecutor::native_version().runtime_version
				)
			} else if config.chain_spec.is_westend() {
				runtime.run_node(
					|config| {
						service::westend_new_light(config)
					},
					|config| {
						service::westend_new_full(
							config,
							None,
							None,
							authority_discovery_enabled,
							6000,
							grandpa_pause
						).map(|(s, _, _)| s)
					},
					service::WestendExecutor::native_version().runtime_version
				)
			} else {
				runtime.run_node(
					|config| {
						service::polkadot_new_light(config)
					},
					|config| {
						service::polkadot_new_full(
							config,
							None,
							None,
							authority_discovery_enabled,
							6000,
							grandpa_pause
						).map(|(s, _, _)| s)
					},
					service::PolkadotExecutor::native_version().runtime_version
				)
			}
		},
		Some(Subcommand::Base(subcommand)) => {
			let runtime = cli.create_runner(subcommand)?;

			if runtime.config().chain_spec.is_kusama() {
				runtime.run_subcommand::<service::kusama_runtime::Runtime, _, _, _>(subcommand, |config|
					service::new_chain_ops::<
						service::kusama_runtime::RuntimeApi,
						service::KusamaExecutor,
						service::kusama_runtime::UncheckedExtrinsic,
					>(config)
				)
			} else if runtime.config().chain_spec.is_westend() {
				runtime.run_subcommand::<service::westend_runtime::Runtime, _, _, _>(subcommand, |config|
					service::new_chain_ops::<
						service::westend_runtime::RuntimeApi,
						service::WestendExecutor,
						service::westend_runtime::UncheckedExtrinsic,
					>(config)
				)
			} else {
				runtime.run_subcommand::<service::polkadot_runtime::Runtime, _, _, _>(subcommand, |config|
					service::new_chain_ops::<
						service::polkadot_runtime::RuntimeApi,
						service::PolkadotExecutor,
						service::polkadot_runtime::UncheckedExtrinsic,
					>(config)
				)
			}
		},
		Some(Subcommand::ValidationWorker(cmd)) => {
			sc_cli::init_logger("");

			if cfg!(feature = "browser") {
				Err(sc_cli::Error::Input("Cannot run validation worker in browser".into()))
			} else {
				#[cfg(not(feature = "browser"))]
				service::run_validation_worker(&cmd.mem_id)?;
				Ok(())
			}
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runtime = cli.create_runner(cmd)?;

			if runtime.config().chain_spec.is_kusama() {
				runtime.sync_run(|config| {
					cmd.run::<service::kusama_runtime::Block, service::KusamaExecutor>(config)
				})
			} else if runtime.config().chain_spec.is_westend() {
				runtime.sync_run(|config| {
					cmd.run::<service::westend_runtime::Block, service::WestendExecutor>(config)
				})
			} else {
				runtime.sync_run(|config| {
					cmd.run::<service::polkadot_runtime::Block, service::PolkadotExecutor>(config)
				})
			}
		},
	}
}
