// The sixteen methods. Included by mod.rs.
//
// Fourteen are three lines each, as `nwc`'s are. The two asynchronous ones
// get two functions apiece — see the module docs.

impl NostrNodeControl {
    // ── channel management ───────────────────────────────────────────

    /// List this node's channels.
    pub async fn list_channels(&self) -> Result<ListChannelsResponse, Error> {
        self.call(Method::ListChannels, ListChannelsRequest {}).await
    }

    /// Open a channel, and wait for it to confirm.
    ///
    /// The response only **acknowledges**; the outcome arrives as a
    /// `channel_opened` notification, which is what the returned handle
    /// resolves to. Await it, spawn it, or drop it.
    ///
    /// If you will not consume the outcome, call
    /// [`open_channel_without_notification`] instead — dropping the handle
    /// still leaves the node sending a notification nobody reads.
    ///
    /// [`open_channel_without_notification`]: Self::open_channel_without_notification
    pub async fn open_channel(
        &self,
        request: OpenChannelRequest,
    ) -> Result<Pending<ChannelOpened>, Error> {
        self.begin(Method::OpenChannel, OpenChannelRequest { notify: Some(true), ..request })
            .await
    }

    /// Open a channel and ask **not** to be told how it went.
    ///
    /// Sends `notify: false`, so the node does not send a notification at
    /// all. This is the honest way to fire and forget: dropping a handle
    /// only means nobody reads the event, not that nobody sends it.
    pub async fn open_channel_without_notification(
        &self,
        request: OpenChannelRequest,
    ) -> Result<(), Error> {
        let _: OpenChannelResponse = self
            .call(
                Method::OpenChannel,
                OpenChannelRequest { notify: Some(false), ..request },
            )
            .await?;
        Ok(())
    }

    /// Close a channel, and wait for the close to confirm.
    pub async fn close_channel(
        &self,
        request: CloseChannelRequest,
    ) -> Result<Pending<ChannelClosed>, Error> {
        self.begin(Method::CloseChannel, CloseChannelRequest { notify: Some(true), ..request })
            .await
    }

    /// Close a channel and ask **not** to be told how it went.
    pub async fn close_channel_without_notification(
        &self,
        request: CloseChannelRequest,
    ) -> Result<(), Error> {
        let _: CloseChannelResponse = self
            .call(
                Method::CloseChannel,
                CloseChannelRequest { notify: Some(false), ..request },
            )
            .await?;
        Ok(())
    }

    // ── peer management ──────────────────────────────────────────────

    /// List connected and known peers.
    pub async fn list_peers(&self) -> Result<ListPeersResponse, Error> {
        self.call(Method::ListPeers, ListPeersRequest {}).await
    }

    /// Connect to a peer.
    pub async fn connect_peer(&self, request: ConnectPeerRequest) -> Result<(), Error> {
        let _: ConnectPeerResponse = self.call(Method::ConnectPeer, request).await?;
        Ok(())
    }

    /// Disconnect from a peer.
    pub async fn disconnect_peer(&self, request: DisconnectPeerRequest) -> Result<(), Error> {
        let _: DisconnectPeerResponse = self.call(Method::DisconnectPeer, request).await?;
        Ok(())
    }

    // ── fees and routing ─────────────────────────────────────────────

    /// Read fee policy.
    pub async fn get_channel_fees(
        &self,
        request: GetChannelFeesRequest,
    ) -> Result<GetChannelFeesResponse, Error> {
        self.call(Method::GetChannelFees, request).await
    }

    /// Set fee policy.
    pub async fn set_channel_fees(&self, request: SetChannelFeesRequest) -> Result<(), Error> {
        let _: SetChannelFeesResponse = self.call(Method::SetChannelFees, request).await?;
        Ok(())
    }

    /// Forwarding history.
    pub async fn get_forwarding_history(
        &self,
        request: GetForwardingHistoryRequest,
    ) -> Result<GetForwardingHistoryResponse, Error> {
        self.call(Method::GetForwardingHistory, request).await
    }

    /// HTLCs in flight.
    pub async fn get_pending_htlcs(&self) -> Result<GetPendingHtlcsResponse, Error> {
        self.call(Method::GetPendingHtlcs, GetPendingHtlcsRequest {}).await
    }

    /// Find routes.
    pub async fn query_routes(
        &self,
        request: QueryRoutesRequest,
    ) -> Result<QueryRoutesResponse, Error> {
        self.call(Method::QueryRoutes, request).await
    }

    // ── network graph ────────────────────────────────────────────────

    /// List graph nodes.
    pub async fn list_network_nodes(
        &self,
        request: ListNetworkNodesRequest,
    ) -> Result<ListNetworkNodesResponse, Error> {
        self.call(Method::ListNetworkNodes, request).await
    }

    /// Aggregate graph statistics.
    pub async fn get_network_stats(&self) -> Result<GetNetworkStatsResponse, Error> {
        self.call(Method::GetNetworkStats, GetNetworkStatsRequest {}).await
    }

    /// One node from the graph.
    pub async fn get_network_node(
        &self,
        request: GetNetworkNodeRequest,
    ) -> Result<GetNetworkNodeResponse, Error> {
        self.call(Method::GetNetworkNode, request).await
    }

    /// One channel from the graph.
    pub async fn get_network_channel(
        &self,
        request: GetNetworkChannelRequest,
    ) -> Result<GetNetworkChannelResponse, Error> {
        self.call(Method::GetNetworkChannel, request).await
    }

    // ── node identity ────────────────────────────────────────────────

    /// Sign a message with the node's **Lightning** identity key.
    pub async fn sign_message(
        &self,
        request: SignMessageRequest,
    ) -> Result<SignMessageResponse, Error> {
        self.call(Method::SignMessage, request).await
    }

    // ── subscriptions ────────────────────────────────────────────────

    /// Set which notification types this controller receives.
    ///
    /// Publishes a kind `30199` addressable event, **replacing** any
    /// previous one — there is no subscribe, resubscribe and unsubscribe,
    /// because a subscription has one piece of state. An empty slice
    /// clears it.
    ///
    /// It authorizes nothing on its own: delivery is the intersection of
    /// this and the `notifications` map in the controller's grant.
    pub async fn set_subscription(&self, types: &[NotificationType]) -> Result<(), Error> {
        self.bootstrap().await?;
        let names: Vec<&str> = types.iter().map(|t| t.as_str()).collect();
        let content = serde_json::to_string(&names)?;
        let event = EventBuilder::new(Kind::Custom(SUBSCRIPTION_KIND), content)
            .tags([
                Tag::identifier(self.uri.service.to_hex()),
                Tag::public_key(self.uri.service),
            ])
            .sign(&self.signer)
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;
        self.client
            .send_event(&event)
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;
        Ok(())
    }

    /// Handle notifications as they arrive, until the callback says stop.
    ///
    /// A **stream**, so a callback is the shape that fits — unbounded and
    /// uncorrelated, unlike an asynchronous command's outcome, which is
    /// exactly one event and gets a [`Pending`] handle instead.
    pub async fn handle_notifications<F, Fut>(&self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(Notification) -> Fut,
        Fut: std::future::Future<Output = Result<bool, Error>>,
    {
        self.bootstrap().await?;
        let me = self.signer.get_public_key().await.map_err(Error::Signer)?;
        self.client
            .subscribe(
                Filter::new()
                    .kind(Kind::Custom(NOTIFICATION_KIND))
                    .pubkey(me)
                    .since(Timestamp::now()),
            )
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;

        let mut notifications = self.client.notifications();
        while let Some(n) = notifications.next().await {
            let ClientNotification::Event { event, .. } = n else { continue };
            if event.kind != Kind::Custom(NOTIFICATION_KIND) || event.pubkey != self.uri.service {
                continue;
            }
            let plaintext = self
                .signer
                .nip44_decrypt(&self.uri.service, &event.content)
                .await
                .map_err(Error::Signer)?;
            let parsed: Notification = serde_json::from_str(&plaintext)?;
            if !f(parsed).await? {
                break;
            }
        }
        Ok(())
    }

    /// Send an asynchronous command: subscribe for its outcome, then send.
    ///
    /// The subscription goes up **first**, or a fast notification arrives
    /// before anything is listening.
    async fn begin<P, T>(&self, method: Method, params: P) -> Result<Pending<T>, Error>
    where
        P: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        self.bootstrap().await?;
        let me = self.signer.get_public_key().await.map_err(Error::Signer)?;

        let request = Request::new(method.clone(), params)?;
        let payload = serde_json::to_string(&request)?;
        let ciphertext = self
            .signer
            .nip44_encrypt(&self.uri.service, &payload)
            .await
            .map_err(Error::Signer)?;
        let event = EventBuilder::new(Kind::Custom(REQUEST_KIND), ciphertext)
            .tag(Tag::public_key(self.uri.service))
            .sign(&self.signer)
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;
        let request_id = event.id;

        // Both subscriptions before the send: the response, and the
        // outcome that will reference this request's id.
        self.client
            .subscribe(
                Filter::new()
                    .kind(Kind::Custom(RESPONSE_KIND))
                    .pubkey(me)
                    .since(Timestamp::now()),
            )
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;
        let outcome_sub = self
            .client
            .subscribe(
                Filter::new()
                    .kind(Kind::Custom(NOTIFICATION_KIND))
                    .pubkey(me)
                    .event(request_id),
            )
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;

        // Two streams, both taken before the send.
        //
        // The relay subscriptions above are not enough: the client
        // broadcasts what it receives, and a receiver taken later never
        // sees what was already delivered. The outcome stream in
        // particular has to exist now rather than when the handle is
        // awaited, or everything the handle is *for* — hold it, spawn it,
        // await it later — silently loses the outcome. Two concurrent
        // opens are the case that finds this: the second command's round
        // trip is the window in which the first one's outcome vanishes.
        let responses = self.client.notifications();
        let outcomes = self.client.notifications();

        self.client
            .send_event(&event)
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;

        // The acknowledgement first — a refusal here means no outcome is
        // coming, so the handle would wait forever.
        let response = self.await_response(request_id, responses).await?;
        if let Some(e) = response.error {
            return Err(Error::Refused(e));
        }

        Ok(Pending::new(
            self.client.clone(),
            self.signer.clone(),
            outcomes,
            outcome_sub.val,
            request_id,
            self.uri.service,
            self.outcome_timeout,
        ))
    }
}
