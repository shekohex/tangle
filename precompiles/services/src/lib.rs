#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use pallet_evm::AddressMapping;
use pallet_services::types::BalanceOf;
use parity_scale_codec::Decode;
use precompile_utils::prelude::*;
use sp_core::U256;
use sp_runtime::traits::Dispatchable;
use sp_runtime::Percent;
use sp_std::{marker::PhantomData, vec::Vec};
use tangle_primitives::services::{Field, OperatorPreferences, ServiceBlueprint};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod mock_evm;
#[cfg(test)]
mod tests;

/// Precompile for the `Services` pallet.
pub struct ServicesPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> ServicesPrecompile<Runtime>
where
	Runtime: pallet_services::Config + pallet_evm::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime::RuntimeCall: From<pallet_services::Call<Runtime>>,
{
	/// Create a new blueprint.
	#[precompile::public("createBlueprint(bytes)")]
	fn create_blueprint(
		handle: &mut impl PrecompileHandle,
		blueprint_data: UnboundedBytes,
	) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let blueprint_data: Vec<u8> = blueprint_data.into();
		let blueprint: ServiceBlueprint<Runtime::Constraints> =
			Decode::decode(&mut &blueprint_data[..])
				.map_err(|_| revert("Invalid blueprint data"))?;

		let call = pallet_services::Call::<Runtime>::create_blueprint { blueprint };

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Register as an operator for a specific blueprint.
	#[precompile::public("registerOperator(uint256,bytes,bytes)")]
	fn register_operator(
		handle: &mut impl PrecompileHandle,
		blueprint_id: U256,
		preferences: UnboundedBytes,
		registration_args: UnboundedBytes,
	) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		// msg.value
		let value = handle.context().apparent_value;

		let blueprint_id: u64 = blueprint_id.as_u64();
		let preferences: Vec<u8> = preferences.into();
		let registration_args: Vec<u8> = registration_args.into();
		let preferences: OperatorPreferences = Decode::decode(&mut &preferences[..])
			.map_err(|_| revert("Invalid preferences data"))?;

		let registration_args: Vec<Field<Runtime::Constraints, Runtime::AccountId>> =
			if registration_args.is_empty() {
				Vec::new()
			} else {
				Decode::decode(&mut &registration_args[..])
					.map_err(|_| revert("Invalid registration arguments"))?
			};
		let value_bytes = {
			let mut value_bytes = [0u8; core::mem::size_of::<U256>()];
			value.to_little_endian(&mut value_bytes);
			value_bytes
		};
		let value = BalanceOf::<Runtime>::decode(&mut &value_bytes[..])
			.map_err(|_| revert("Value is not a valid balance"))?;
		let call = pallet_services::Call::<Runtime>::register {
			blueprint_id,
			preferences,
			registration_args,
			value,
		};

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Unregister as an operator from a blueprint.
	#[precompile::public("unregisterOperator(uint256)")]
	fn unregister_operator(handle: &mut impl PrecompileHandle, blueprint_id: U256) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let blueprint_id: u64 = blueprint_id.as_u64();

		let call = pallet_services::Call::<Runtime>::unregister { blueprint_id };

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Request a new service.
	#[precompile::public("requestService(uint256,uint256[],bytes,bytes,bytes)")]
	fn request_service(
		handle: &mut impl PrecompileHandle,
		blueprint_id: U256,
		assets: Vec<U256>,
		permitted_callers_data: UnboundedBytes,
		service_providers_data: UnboundedBytes,
		request_args_data: UnboundedBytes,
	) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let blueprint_id: u64 = blueprint_id.as_u64();
		let permitted_callers_data: Vec<u8> = permitted_callers_data.into();
		let service_providers_data: Vec<u8> = service_providers_data.into();
		let request_args_data: Vec<u8> = request_args_data.into();

		let permitted_callers: Vec<Runtime::AccountId> =
			Decode::decode(&mut &permitted_callers_data[..])
				.map_err(|_| revert("Invalid permitted callers data"))?;

		let operators: Vec<Runtime::AccountId> =
			Decode::decode(&mut &service_providers_data[..])
				.map_err(|_| revert("Invalid service providers data"))?;

		let request_args: Vec<Field<Runtime::Constraints, Runtime::AccountId>> =
			Decode::decode(&mut &request_args_data[..])
				.map_err(|_| revert("Invalid request arguments data"))?;
		let assets: Vec<Runtime::AssetId> =
			assets.into_iter().map(|asset| asset.as_u32().into()).collect();

		let value_bytes = {
			let value = handle.context().apparent_value;
			let mut value_bytes = [0u8; core::mem::size_of::<U256>()];
			value.to_little_endian(&mut value_bytes);
			value_bytes
		};
		let value = BalanceOf::<Runtime>::decode(&mut &value_bytes[..])
			.map_err(|_| revert("Value is not a valid balance"))?;
		let call = pallet_services::Call::<Runtime>::request {
			blueprint_id,
			permitted_callers,
			operators,
			ttl: 10000_u32.into(),
			assets,
			request_args,
			value,
		};

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Terminate a service.
	#[precompile::public("terminateService(uint256)")]
	fn terminate_service(handle: &mut impl PrecompileHandle, service_id: U256) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let service_id: u64 = service_id.as_u64();

		let call = pallet_services::Call::<Runtime>::terminate { service_id };

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Approve a request.
	#[precompile::public("approve(uint256,uint8)")]
	fn approve(
		handle: &mut impl PrecompileHandle,
		request_id: U256,
		restaking_percent: u8,
	) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let request_id: u64 = request_id.as_u64();
		let restaking_percent: Percent = Percent::from_percent(restaking_percent);

		let call = pallet_services::Call::<Runtime>::approve { request_id, restaking_percent };

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Reject a service request.
	#[precompile::public("reject(uint256)")]
	fn reject(handle: &mut impl PrecompileHandle, request_id: U256) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let request_id: u64 = request_id.as_u64();

		let call = pallet_services::Call::<Runtime>::reject { request_id };

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Terminate a service by the owner of the service.
	#[precompile::public("terminate(uint256)")]
	fn terminate(handle: &mut impl PrecompileHandle, service_id: U256) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let service_id: u64 = service_id.as_u64();

		let call = pallet_services::Call::<Runtime>::terminate { service_id };

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Call a job in the service.
	#[precompile::public("callJob(uint256,uint8,bytes)")]
	fn call_job(
		handle: &mut impl PrecompileHandle,
		service_id: U256,
		job: u8,
		args_data: UnboundedBytes,
	) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let service_id: u64 = service_id.as_u64();
		let args: Vec<u8> = args_data.into();

		let decoded_args: Vec<Field<Runtime::Constraints, Runtime::AccountId>> =
			Decode::decode(&mut &args[..])
				.map_err(|_| revert("Invalid job call arguments data"))?;

		let call = pallet_services::Call::<Runtime>::call { service_id, job, args: decoded_args };

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Submit the result for a job call.
	#[precompile::public("submitResult(uint256,uint256,bytes)")]
	fn submit_result(
		handle: &mut impl PrecompileHandle,
		service_id: U256,
		call_id: U256,
		result_data: UnboundedBytes,
	) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let service_id: u64 = service_id.as_u64();
		let call_id: u64 = call_id.as_u64();
		let result: Vec<u8> = result_data.into();

		let decoded_result: Vec<Field<Runtime::Constraints, Runtime::AccountId>> =
			Decode::decode(&mut &result[..]).map_err(|_| revert("Invalid job result data"))?;

		let call = pallet_services::Call::<Runtime>::submit_result {
			service_id,
			call_id,
			result: decoded_result,
		};

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Slash an operator (offender) for a service id with a given percent of their exposed stake for that service.
	///
	/// The caller needs to be an authorized Slash Origin for this service.
	/// Note that this does not apply the slash directly, but instead schedules a deferred call to apply the slash
	/// by another entity.
	#[precompile::public("slash(bytes,uint256,uint8)")]
	fn slash(
		handle: &mut impl PrecompileHandle,
		offender: UnboundedBytes,
		service_id: U256,
		percent: u8,
	) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller = handle.context().caller;
		let origin = Runtime::AddressMapping::into_account_id(caller);
		let service_id: u64 = service_id.as_u64();
		let percent: Percent = Percent::from_percent(percent);
		let offender_bytes: Vec<_> = offender.into();
		let offender: Runtime::AccountId = Decode::decode(&mut &offender_bytes[..])
			.map_err(|_| revert("Invalid offender account id"))?;

		// inside this call, we do check if the caller is authorized to slash the offender
		let call = pallet_services::Call::<Runtime>::slash { offender, service_id, percent };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}

	/// Dispute an Unapplied Slash for a service id.
	///
	/// The caller needs to be an authorized Dispute Origin for this service.
	#[precompile::public("dispute(uint32,uint32)")]
	fn dispute(handle: &mut impl PrecompileHandle, era: u32, index: u32) -> EvmResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller = handle.context().caller;
		let origin = Runtime::AddressMapping::into_account_id(caller);

		// inside this call, we do check if the caller is authorized to dispute the slash
		let call = pallet_services::Call::<Runtime>::dispute { era, index };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}
}
