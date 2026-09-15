//! One thin function per method.
//!
//! Each is the same three lines — build params, `send`, decode — and the
//! value of writing them out is that a caller gets a typed request and a
//! typed response rather than a `Value` and a convention.
//!
//! Not every method the crate types appears here. These are the ones a
//! **party in a trade** needs, which is what this client was written for;
//! adding the rest is mechanical and should happen when something needs
//! them rather than in advance.

use super::{Error, WalletConnect};
use crate::nwc::methods::*;
use crate::nwc::WalletMethod;

/// Decode a result, or say it did not fit.
fn decode<T: serde::de::DeserializeOwned>(
    _method: WalletMethod,
    v: serde_json::Value,
) -> Result<T, Error> {
    serde_json::from_value(v).map_err(|e| Error::Result(crate::nnc::ResultError::Malformed(e)))
}

impl WalletConnect {
    /// What the wallet is and does.
    pub async fn get_info(&self) -> Result<GetInfoResponse, Error> {
        let v = self.send(WalletMethod::GetInfo, serde_json::json!({})).await?;
        decode(WalletMethod::GetInfo, v)
    }

    /// The wallet's balance.
    pub async fn get_balance(&self) -> Result<GetBalanceResponse, Error> {
        let v = self.send(WalletMethod::GetBalance, serde_json::json!({})).await?;
        decode(WalletMethod::GetBalance, v)
    }

    /// Create an invoice.
    ///
    /// **The wallet generates the preimage and keeps it.** That is what
    /// makes this an *ordinary* invoice: it settles on receipt, and the
    /// caller never has the secret to withhold.
    pub async fn make_invoice(&self, r: MakeInvoiceRequest) -> Result<MakeInvoiceResponse, Error> {
        let v = self.send(WalletMethod::MakeInvoice, serde_json::to_value(&r)?).await?;
        decode(WalletMethod::MakeInvoice, v)
    }

    /// Look one up.
    pub async fn lookup_invoice(
        &self,
        r: LookupInvoiceRequest,
    ) -> Result<LookupInvoiceResponse, Error> {
        let v = self.send(WalletMethod::LookupInvoice, serde_json::to_value(&r)?).await?;
        decode(WalletMethod::LookupInvoice, v)
    }

    /// Pay a BOLT11 invoice.
    ///
    /// **Against a hold invoice this does not return promptly.** The HTLC
    /// is accepted and then nothing happens until the payee settles —
    /// seconds if all is well, or the full timeout if they walk away. Use
    /// [`WalletConnect::with_timeout`] or drive it from its own task.
    pub async fn pay_invoice(&self, r: PayInvoiceRequest) -> Result<PayInvoiceResponse, Error> {
        let v = self.send(WalletMethod::PayInvoice, serde_json::to_value(&r)?).await?;
        decode(WalletMethod::PayInvoice, v)
    }

    /// Create a hold invoice for a hash generated elsewhere. NWC-03.
    ///
    /// Takes a payment hash and never a preimage: the secret belongs to
    /// whoever made it, which is what lets this lock to a payment someone
    /// else will settle.
    pub async fn make_hold_invoice(
        &self,
        r: MakeHoldInvoiceRequest,
    ) -> Result<MakeHoldInvoiceResponse, Error> {
        let v = self.send(WalletMethod::MakeHoldInvoice, serde_json::to_value(&r)?).await?;
        decode(WalletMethod::MakeHoldInvoice, v)
    }

    /// Settle one with the preimage. NWC-03.
    pub async fn settle_hold_invoice(
        &self,
        r: SettleHoldInvoiceRequest,
    ) -> Result<SettleHoldInvoiceResponse, Error> {
        let v = self.send(WalletMethod::SettleHoldInvoice, serde_json::to_value(&r)?).await?;
        decode(WalletMethod::SettleHoldInvoice, v)
    }

    /// Cancel one, releasing the payer. NWC-03.
    pub async fn cancel_hold_invoice(
        &self,
        r: CancelHoldInvoiceRequest,
    ) -> Result<CancelHoldInvoiceResponse, Error> {
        let v = self.send(WalletMethod::CancelHoldInvoice, serde_json::to_value(&r)?).await?;
        decode(WalletMethod::CancelHoldInvoice, v)
    }

    /// What a payment would cost, without sending it. `nwc-route.md`.
    ///
    /// What a maker needs before quoting a firm price: it cannot discover
    /// the cost by attempting the payment, because attempting it is
    /// precisely what it must not do until it has been paid.
    pub async fn quote_payment(&self, r: QuotePaymentRequest) -> Result<QuotePaymentResponse, Error> {
        let v = self.send(WalletMethod::QuotePayment, serde_json::to_value(&r)?).await?;
        decode(WalletMethod::QuotePayment, v)
    }
}
