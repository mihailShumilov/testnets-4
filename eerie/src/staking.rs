use super::{Eerie, SimpleCoin};
use orga::coins::*;
use orga::prelude::*;

#[derive(State)]
pub struct AppWithStaking {
    height: u64,
    pub simp: SimpleCoin,
    staking: Staking,
}

impl AppWithStaking {
    pub fn delegate(&mut self, validator_address: Address, amount: Amount<Eerie>) -> Result<()> {
        let signer = self
            .context::<Signer>()
            .ok_or_else(|| failure::format_err!("No signer context available"))?
            .signer
            .ok_or_else(|| failure::format_err!("Delegate calls must be signed"))?;

        let mut sender = self.simp.balances_mut().entry(signer)?.or_default()?;
        let coins = sender.take(amount)?;
        let mut validator = self.staking.validators.get_mut(validator_address)?;
        validator.get_mut(signer)?.give(coins)?;
        Ok(())
    }
}

impl EndBlock for AppWithStaking {
    fn end_block(&mut self, ctx: &EndBlockCtx) -> Result<()> {
        // Pop front of unbonding queue until we've paid out all the mature
        // unbonds
        while let Some(unbond) = self.staking.unbonding_queue.front()? {
            if unbond.maturity_height <= self.height {
                let unbond = self.staking.unbonding_queue.pop_front()?.unwrap();
                let mut unbonder_account = self
                    .simp
                    .balances_mut()
                    .entry(unbond.delegator_address)?
                    .or_default()?;
                unbonder_account.add(unbond.coin.amount)?;

                let validator_address = unbond.validator_address;
                let validator = self.staking.validators.get(validator_address)?;

                let new_voting_power = validator.balance().value;
                ctx.set_voting_power(validator_address.bytes(), new_voting_power);
            }
        }

        Ok(())
    }
}

impl BeginBlock for AppWithStaking {
    fn begin_block(&mut self, ctx: &BeginBlockCtx) -> Result<()> {
        self.height = ctx.height;
        let block_reward = Eerie::mint(10);
        self.staking.validators.give(block_reward)
    }
}

#[derive(State)]
pub struct Unbond {
    pub coin: Coin<Eerie>,
    pub delegator_address: Address,
    pub validator_address: Address,
    pub maturity_height: u64,
}

type Delegators = Pool<Address, Coin<Eerie>, Eerie>;
#[derive(State)]
pub struct Staking {
    pub validators: Pool<Address, Delegators, Eerie>,
    pub unbonding_queue: Deque<Unbond>,
}
