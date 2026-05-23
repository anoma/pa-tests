use anoma_rm_risc0::transaction::Transaction as ArmTxn;

use super::Transaction;

impl From<Transaction> for ArmTxn {
    #[inline]
    fn from(transaction: Transaction) -> Self {
        transaction.arm_txn
    }
}
