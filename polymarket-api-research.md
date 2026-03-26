# Polymarket CLOB API -- Technical Research for Trading Bot Adapter

## Table of Contents
1. [Architecture Overview](#1-architecture-overview)
2. [Base URLs & Endpoints](#2-base-urls--endpoints)
3. [Authentication](#3-authentication)
4. [Order Types & Trading](#4-order-types--trading)
5. [Market Structure](#5-market-structure)
6. [REST API -- Complete Endpoint Reference](#6-rest-api----complete-endpoint-reference)
7. [WebSocket Feeds](#7-websocket-feeds)
8. [Settlement & On-Chain Mechanics](#8-settlement--on-chain-mechanics)
9. [Rate Limits](#9-rate-limits)
10. [On-Chain Contracts & Copy Trading](#10-on-chain-contracts--copy-trading)
11. [SDK Clients & GitHub Repos](#11-sdk-clients--github-repos)

---

## 1. Architecture Overview

Polymarket operates as a **hybrid-decentralized trading system**:
- **Off-chain order matching** via a Central Limit Order Book (CLOB)
- **On-chain settlement** on Polygon (Chain ID 137) using EIP-712 signed messages
- The operator cannot unilaterally set prices or execute unauthorized trades
- All positions are non-custodial -- users retain control of funds through signed order messages

The platform has **four distinct API services**:

| API | Base URL | Purpose |
|-----|----------|---------|
| **Gamma API** | `https://gamma-api.polymarket.com` | Market discovery, events, tags, search, metadata |
| **CLOB API** | `https://clob.polymarket.com` | Order book data, pricing, order management, trading |
| **Data API** | `https://data-api.polymarket.com` | User positions, trades, activity, leaderboards, analytics |
| **Bridge API** | `https://bridge.polymarket.com` | Deposit/withdrawal operations |

---

## 2. Base URLs & Endpoints

### CLOB API Endpoint Paths (Base: `https://clob.polymarket.com`)

#### Public (No Auth)
```
GET  /                              Health check
GET  /time                          Server timestamp
GET  /book?token_id={id}            Order book for a token
POST /books                         Order books for multiple tokens (body: [{token_id}])
GET  /price?token_id={id}&side={s}  Price for a token (side=BUY|SELL)
POST /prices                        Prices for multiple tokens
GET  /midpoint?token_id={id}        Midpoint price
POST /midpoints                     Midpoints for multiple tokens
GET  /spread?token_id={id}          Bid-ask spread
POST /spreads                       Spreads for multiple tokens
GET  /last-trade-price?token_id={id}         Last trade price
POST /last-trades-prices                      Last trade prices (batch)
GET  /tick-size?token_id={id}                 Tick size for market
GET  /neg-risk?token_id={id}                  Whether market is neg-risk
GET  /fee-rate?token_id={id}                  Fee rate in basis points
GET  /prices-history?market={id}&interval={i} Historical prices
GET  /markets?next_cursor={c}                 All CLOB markets (paginated)
GET  /markets/{condition_id}                  Single market by condition ID
GET  /simplified-markets?next_cursor={c}      Lightweight market listing
GET  /sampling-markets?next_cursor={c}        Reward-eligible markets
GET  /sampling-simplified-markets?next_cursor={c}
GET  /live-activity/events/{condition_id}     Market trade events
```

#### Auth Required -- L1 (Private Key Signature)
```
POST /auth/api-key                  Create API key
GET  /auth/derive-api-key           Derive existing API key
```

#### Auth Required -- L2 (HMAC + API Key)
```
GET  /auth/api-keys                 List API keys
DELETE /auth/api-key                Delete API key
GET  /auth/ban-status/closed-only   Check ban status
POST /auth/readonly-api-key         Create readonly API key
GET  /auth/readonly-api-keys        List readonly API keys
DELETE /auth/readonly-api-key       Delete readonly API key
GET  /auth/validate-readonly-api-key?address={a}&key={k}  Validate (public)

POST /order                         Place a single order
POST /orders                        Place batch orders (up to 15)
DELETE /order                       Cancel single order (body: {orderID})
DELETE /orders                      Cancel multiple orders (body: [orderID,...])
DELETE /cancel-all                  Cancel all orders
DELETE /cancel-market-orders        Cancel orders for a market (body: {market, asset_id})

GET  /data/order/{order_id}         Get order by ID
GET  /data/orders?next_cursor={c}   Get open orders (paginated)
GET  /data/trades?next_cursor={c}   Get trade history (paginated)

GET  /balance-allowance             Get balance & allowance
GET  /balance-allowance/update      Update balance & allowance
GET  /order-scoring?orderId={id}    Check if order is scoring (rewards)
POST /orders-scoring                Check if orders are scoring (batch)

GET  /notifications                 Get notifications
DELETE /notifications               Drop notifications

POST /v1/heartbeats                 Send heartbeat (auto-cancel if missed)
```

### Gamma API Endpoint Paths (Base: `https://gamma-api.polymarket.com`)
```
GET  /events                        List events (filterable, paginated)
GET  /events/{id}                   Get event by ID
GET  /events/slug/{slug}            Get event by slug
GET  /markets                       List markets (filterable, paginated)
GET  /markets/{id}                  Get market by ID
GET  /markets/slug/{slug}           Get market by slug
GET  /tags                          List category tags
GET  /sports                        Sports metadata with tag IDs
GET  /series                        Grouped event series
GET  /teams                         Team information
GET  /public-search                 Search events, markets, profiles
```

### Data API Endpoint Paths (Base: `https://data-api.polymarket.com`)
```
GET  /positions?user={address}      Current positions for a user
GET  /closed-positions?user={addr}  Historical closed positions
GET  /activity?user={address}       On-chain activity (trades, splits, merges, redeems)
GET  /value?user={address}          Total position value (USD)
GET  /trades                        Trade history
GET  /oi?market={condition_id}      Open interest for a market
GET  /holders?market={condition_id} Top holders of a market
```

---

## 3. Authentication

### Three Authentication Levels

**Level 0 (L0)** -- No auth. Public endpoints only (market data, prices, order books).

**Level 1 (L1)** -- Private key + EIP-712 signature. Used to create/derive API credentials.

**Level 2 (L2)** -- HMAC-SHA256 with derived API credentials. Required for all trading operations.

### L1 Authentication Headers
Used for creating or deriving API keys:

```
POLY_ADDRESS:   <wallet_address>
POLY_SIGNATURE: <eip712_signature>
POLY_TIMESTAMP: <unix_timestamp_seconds>
POLY_NONCE:     <nonce_integer>
```

The EIP-712 message signed:
- Domain: `{name: "ClobAuthDomain", version: "1", chainId: 137}`
- Message: `{address, timestamp, nonce, message: "This message attests that I control the given wallet"}`

### L2 Authentication Headers
Used for all trading operations:

```
POLY_ADDRESS:    <wallet_address>
POLY_SIGNATURE:  <hmac_sha256_signature>
POLY_TIMESTAMP:  <unix_timestamp_seconds>
POLY_API_KEY:    <api_key>
POLY_PASSPHRASE: <api_passphrase>
```

### HMAC Signature Construction
```python
message = str(timestamp) + str(method) + str(request_path)
if body:
    message += str(body)  # JSON serialized, double quotes

secret_bytes = base64.urlsafe_b64decode(api_secret)
signature = base64.urlsafe_b64encode(
    hmac.new(secret_bytes, message.encode('utf-8'), hashlib.sha256).digest()
)
```

### API Credentials Structure
When you create or derive API keys, you receive:
```json
{
  "apiKey": "string",
  "secret": "string (base64url encoded)",
  "passphrase": "string"
}
```

### Signature Types
| Type | ID | Use Case | Funder Param |
|------|----|----|--------------|
| EOA (MetaMask, hardware wallet) | 0 | Direct private key control | Own address |
| POLY_PROXY (Magic/email wallet) | 1 | Delegated signing | Proxy wallet address |
| GNOSIS_SAFE (browser wallet proxy) | 2 | Proxy contract | Proxy wallet address |

### Client Initialization Example (Python)
```python
from py_clob_client.client import ClobClient

# Level 0 -- read-only
client = ClobClient("https://clob.polymarket.com")

# Level 2 -- full trading
client = ClobClient(
    "https://clob.polymarket.com",
    key="<private_key>",
    chain_id=137,
    signature_type=0,      # 0=EOA, 1=Magic, 2=Browser proxy
    funder="<funder_addr>"  # only needed for proxy wallets
)
client.set_api_creds(client.create_or_derive_api_creds())
```

---

## 4. Order Types & Trading

### Supported Order Types

| Type | Code | Description |
|------|------|-------------|
| **GTC** | `"GTC"` | Good Till Cancelled -- rests on the book until filled or cancelled |
| **GTD** | `"GTD"` | Good Till Date -- expires at a specified timestamp |
| **FOK** | `"FOK"` | Fill or Kill -- must fill entirely immediately or cancel completely |
| **FAK** | `"FAK"` | Fill and Kill -- fills as much as possible immediately, cancels remainder |

### Limit Order Parameters (`OrderArgs`)
```python
OrderArgs(
    token_id="<token_id>",      # ERC1155 token ID for the outcome
    price=0.55,                  # Price in USD (0.00 to 1.00)
    size=100.0,                  # Number of outcome token shares
    side="BUY",                  # "BUY" or "SELL"
    fee_rate_bps=0,              # Fee rate (auto-resolved from market)
    nonce=0,                     # For on-chain cancellation
    expiration=0,                # Unix timestamp (0 = no expiry, used with GTD)
    taker="0x0000..."            # Zero address = public order
)
```

### Market Order Parameters (`MarketOrderArgs`)
```python
MarketOrderArgs(
    token_id="<token_id>",
    amount=25.0,                 # BUY: dollar amount to spend; SELL: shares to sell
    side="BUY",                  # "BUY" or "SELL"
    price=0,                     # 0 = auto-calculated from order book
    fee_rate_bps=0,
    nonce=0,
    taker="0x0000...",
    order_type=OrderType.FOK     # Market orders should use FOK or FAK
)
```

### Placing an Order
```python
# Limit order
order_args = OrderArgs(token_id="<id>", price=0.50, size=10.0, side="BUY")
signed_order = client.create_order(order_args)
response = client.post_order(signed_order, OrderType.GTC)

# Market order
market_args = MarketOrderArgs(token_id="<id>", amount=25.0, side="BUY",
                               order_type=OrderType.FOK)
signed_order = client.create_market_order(market_args)
response = client.post_order(signed_order, OrderType.FOK)
```

### Post Order Request Body (JSON sent to `POST /order`)
```json
{
  "order": {
    "salt": "...",
    "maker": "0x...",
    "signer": "0x...",
    "taker": "0x0000000000000000000000000000000000000000",
    "tokenId": "...",
    "makerAmount": "...",
    "takerAmount": "...",
    "expiration": "0",
    "nonce": "0",
    "feeRateBps": "0",
    "side": "BUY",
    "signatureType": 0,
    "signature": "0x..."
  },
  "owner": "0x...",
  "orderType": "GTC",
  "postOnly": false
}
```

### Post-Only Orders
- Only valid for GTC and GTD order types
- If `postOnly=true`, the order will only be placed if it would rest on the book (not immediately matched)
- Throws exception if used with FOK or FAK

### Tick Sizes
Markets have minimum tick sizes that constrain valid prices:
- `"0.1"` -- prices at 0.1 increments
- `"0.01"` -- prices at 0.01 increments (most common)
- `"0.001"` -- prices at 0.001 increments (near 0 or 1)
- `"0.0001"` -- finest granularity

Tick size changes dynamically: when price exceeds 0.96 or drops below 0.04, tick size tightens.

### Cancellation
```python
client.cancel("order_id")                          # Single order
client.cancel_orders(["id1", "id2"])                # Multiple orders
client.cancel_all()                                 # All orders
client.cancel_market_orders(market="0x...", asset_id="...")  # By market
```

### Heartbeat Mechanism
If you start sending heartbeats, you MUST continue every 10 seconds or all orders are cancelled:
```python
client.post_heartbeat("heartbeat_id_string")  # POST /v1/heartbeats
```

### Batch Orders
Up to 15 orders per batch call (`POST /orders`).

---

## 5. Market Structure

### Hierarchy: Events > Markets > Tokens

```
Event (container)
  |-- Market 1 (binary question)
  |     |-- Yes Token (ERC1155 token ID)
  |     |-- No Token  (ERC1155 token ID)
  |-- Market 2 (binary question)
  |     |-- Yes Token
  |     |-- No Token
  ...
```

### Key Identifiers

| Identifier | Description |
|------------|-------------|
| **Condition ID** | Unique hash identifying the market's condition in CTF contracts. Used as the market's primary key. |
| **Question ID** | Hash of the UMA ancillary data (the market question text). Used for resolution. |
| **Token ID** | ERC1155 token ID for trading on the CLOB. Each market has TWO (Yes and No). These are very long integers. |
| **Slug** | URL-friendly identifier (e.g., `fed-decision-in-october`) |

### Token ID Computation (on-chain)
1. `conditionId = getConditionId(oracle_address, questionId, outcomeSlotCount=2)`
2. `collectionId = getCollectionId(parentCollectionId=bytes32(0), conditionId, indexSet)` where indexSet=1 for Yes, indexSet=2 for No
3. `positionId = getPositionId(collateral_token_address, collectionId)`

In practice, you retrieve token IDs from the Gamma API (`GET /markets` or `GET /events`) rather than computing them.

### Market Types

**Standard Markets**: Simple binary outcomes. Use CTF Exchange contract.

**Neg Risk Markets**: Multi-outcome events where markets are mutually exclusive (e.g., "Who wins the election?" with separate markets for each candidate). Use Neg Risk CTF Exchange. The `neg_risk` flag is `true`.

### Gamma API Market Response Fields
Key fields returned by `GET /markets`:
- `condition_id`, `question_id`, `question` (text)
- `slug`, `description`, `end_date_iso`
- `active` (bool), `closed` (bool), `archived` (bool)
- `enable_order_book` (bool -- must be true for CLOB trading)
- `minimum_order_size`, `minimum_tick_size`
- `neg_risk` (bool)
- `tokens` -- array of `{token_id, outcome}` (typically "Yes" and "No")
- `rewards` -- liquidity reward parameters
- `volume`, `volume_24hr`, `liquidity`

### Querying Markets

**By slug:**
```
GET https://gamma-api.polymarket.com/events?slug=fed-decision-in-october
GET https://gamma-api.polymarket.com/events/slug/fed-decision-in-october
```

**All active markets:**
```
GET https://gamma-api.polymarket.com/events?active=true&closed=false&order=volume_24hr&ascending=false&limit=50&offset=0
```

**By tag/category:**
```
GET https://gamma-api.polymarket.com/tags           # discover tag IDs
GET https://gamma-api.polymarket.com/events?tag_id=123&active=true
```

**Pagination:** Uses `limit` and `offset` parameters. CLOB API uses cursor-based pagination with `next_cursor` (starts at `"MA=="`, ends at `"LTE="`).

---

## 6. REST API -- Complete Endpoint Reference

### CLOB API -- Market Data (Public)

#### GET /book
**Order book for a single token.**
```
GET https://clob.polymarket.com/book?token_id={token_id}
```
Response:
```json
{
  "market": "0x...(condition_id)",
  "asset_id": "71321045...(token_id)",
  "timestamp": "1700000000",
  "bids": [{"price": "0.48", "size": "30"}, {"price": "0.47", "size": "50"}],
  "asks": [{"price": "0.52", "size": "25"}, {"price": "0.53", "size": "60"}],
  "min_order_size": "5",
  "neg_risk": false,
  "tick_size": "0.01",
  "last_trade_price": "0.50",
  "hash": "0x..."
}
```

#### GET /price
```
GET https://clob.polymarket.com/price?token_id={id}&side=BUY
```
Returns best price as a string.

#### GET /midpoint
```
GET https://clob.polymarket.com/midpoint?token_id={id}
```
Returns midpoint price (average of best bid and best ask).

#### GET /spread
```
GET https://clob.polymarket.com/spread?token_id={id}
```
Returns spread value (best ask - best bid).

#### GET /prices-history
```
GET https://clob.polymarket.com/prices-history?market={token_id}&interval=1d&fidelity=60
```
Parameters:
- `market` -- token ID (required)
- `interval` -- `"max"`, `"1w"`, `"1d"`, `"6h"`, `"1h"` (required)
- `fidelity` -- granularity in minutes (optional)
- `startTs` -- start timestamp (optional)
- `endTs` -- end timestamp (optional)

Response:
```json
[
  {"t": 1700000000, "p": "0.55"},
  {"t": 1700003600, "p": "0.57"}
]
```

### Data API -- User & Market Analytics (Public, No Auth)

#### GET /positions
```
GET https://data-api.polymarket.com/positions?user={address}&limit=100&offset=0
```
Parameters:
- `user` (required) -- wallet address
- `market` -- condition ID(s), comma-separated
- `sizeThreshold` -- minimum position size (default 1.0)
- `redeemable` -- bool filter
- `mergeable` -- bool filter
- `limit` -- max 500 (default 100)
- `offset` -- pagination offset
- `sortBy` -- `TOKENS`, `CURRENT`, `INITIAL`, `CASHPNL`, `PERCENTPNL`, `TITLE`, `RESOLVING`, `PRICE`
- `sortDirection` -- `ASC` or `DESC`

Response fields: `proxyWallet, asset, conditionId, size, avgPrice, initialValue, currentValue, cashPnl, percentPnl, totalBought, realizedPnl, curPrice, redeemable, title, slug, outcome, outcomeIndex, endDate, negativeRisk`

#### GET /trades
```
GET https://data-api.polymarket.com/trades?user={address}&limit=100
```
Parameters:
- `user` -- wallet address
- `market` -- condition ID(s)
- `side` -- `BUY` or `SELL`
- `limit` -- max 500
- `offset` -- pagination
- `takerOnly` -- bool (default true)
- `filterType` -- `CASH` or `TOKENS`
- `filterAmount` -- threshold

Response fields: `proxyWallet, side, asset, conditionId, size, price, timestamp, title, slug, outcome, outcomeIndex, transactionHash`

#### GET /activity
```
GET https://data-api.polymarket.com/activity?user={address}&type=TRADE&limit=100
```
Parameters:
- `user` (required) -- wallet address
- `type` -- comma-separated: `TRADE`, `SPLIT`, `MERGE`, `REDEEM`, `REWARD`, `CONVERSION`
- `market` -- condition ID(s)
- `start`, `end` -- timestamps in seconds
- `side` -- `BUY` or `SELL`
- `sortBy` -- `TIMESTAMP`, `TOKENS`, `CASH`
- `sortDirection` -- `ASC` or `DESC`

#### GET /holders
```
GET https://data-api.polymarket.com/holders?market={condition_id}&limit=100
```

#### GET /oi
```
GET https://data-api.polymarket.com/oi?market={condition_id}
```

#### GET /value
```
GET https://data-api.polymarket.com/value?user={address}
```

---

## 7. WebSocket Feeds

### Connection Endpoints

| Channel | URL | Auth Required |
|---------|-----|---------------|
| **Market** | `wss://ws-subscriptions-clob.polymarket.com/ws/market` | No |
| **User** | `wss://ws-subscriptions-clob.polymarket.com/ws/user` | Yes |
| **Sports** | `wss://sports-api.polymarket.com/ws` | No |
| **RTDS** | `wss://ws-live-data.polymarket.com` | Optional |

### Market Channel

**Subscribe:**
```json
{
  "assets_ids": ["<token_id_1>", "<token_id_2>"],
  "type": "market",
  "custom_feature_enabled": true
}
```

**Dynamic subscribe/unsubscribe (without reconnecting):**
```json
{
  "assets_ids": ["<new_token_id>"],
  "operation": "subscribe",
  "custom_feature_enabled": true
}
```
```json
{
  "assets_ids": ["<token_id>"],
  "operation": "unsubscribe"
}
```

**Event types:**

| Event | Trigger | Key Fields |
|-------|---------|------------|
| `book` | On subscribe + on trade | `asset_id, market, bids[], asks[], timestamp, hash` |
| `price_change` | Order placed/cancelled | `market, price_changes[{asset_id, price, size, side, best_bid, best_ask}], timestamp` |
| `tick_size_change` | Price crosses 0.96/0.04 | `asset_id, old_tick_size, new_tick_size` |
| `last_trade_price` | Trade executed | `asset_id, price, size, side, fee_rate_bps` |
| `best_bid_ask` | BBO changes (requires custom_feature_enabled) | `asset_id, best_bid, best_ask, spread` |
| `new_market` | New market created (requires custom_feature_enabled) | Market metadata |
| `market_resolved` | Market resolved (requires custom_feature_enabled) | Resolution data |

**Example payloads:**

Book:
```json
{
  "event_type": "book",
  "asset_id": "65818619...",
  "market": "0xbd31dc8a...",
  "bids": [{"price": ".48", "size": "30"}, {"price": ".49", "size": "20"}],
  "asks": [{"price": ".52", "size": "25"}, {"price": ".53", "size": "60"}],
  "timestamp": "123456789000",
  "hash": "0x0...."
}
```

Price change:
```json
{
  "event_type": "price_change",
  "market": "0x5f65...",
  "price_changes": [
    {
      "asset_id": "71321045...",
      "price": "0.5",
      "size": "200",
      "side": "BUY",
      "hash": "56621a...",
      "best_bid": "0.5",
      "best_ask": "1"
    }
  ],
  "timestamp": "1757908892351"
}
```

Last trade price:
```json
{
  "event_type": "last_trade_price",
  "asset_id": "114122071...",
  "market": "0x6a67b9d8...",
  "price": "0.456",
  "side": "BUY",
  "size": "219.217767",
  "fee_rate_bps": "0",
  "timestamp": "1750428146322"
}
```

Best bid/ask:
```json
{
  "event_type": "best_bid_ask",
  "market": "0x0005c0d3...",
  "asset_id": "85354956...",
  "best_bid": "0.73",
  "best_ask": "0.77",
  "spread": "0.04",
  "timestamp": "1766789469958"
}
```

### User Channel

**Subscribe (requires L2 credentials):**
```json
{
  "auth": {
    "apiKey": "your-api-key",
    "secret": "your-api-secret",
    "passphrase": "your-passphrase"
  },
  "markets": ["0x1234...condition_id"],
  "type": "user"
}
```

Note: User channel subscribes by **condition IDs** (not token IDs). Each market has one condition ID but two token IDs.

**Event types:**
- `trade` -- Trade lifecycle updates (MATCHED -> CONFIRMED)
- `order` -- Order placements, updates, and cancellations

### Heartbeat Protocol

**Market & User channels:** Client must send `PING` every 10 seconds. Server responds with `PONG`.

**Sports channel:** Server sends `ping` every 5 seconds. Client must respond with `pong` within 10 seconds or the connection is dropped.

---

## 8. Settlement & On-Chain Mechanics

### Conditional Token Framework (CTF)

All outcomes are tokenized using the **Gnosis Conditional Token Framework**:
- Creates **ERC1155** tokens representing market outcomes
- Each binary market has two tokens (Yes / No)
- Every Yes/No pair is backed by exactly **$1.00 USDC.e** locked in the CTF contract
- Prices range from $0.00 to $1.00

### Core On-Chain Operations

| Operation | Description |
|-----------|-------------|
| **Split** | Deposit USDC.e -> receive paired Yes + No tokens |
| **Merge** | Return Yes + No token pair -> receive USDC.e back |
| **Redeem** | After resolution, exchange winning tokens for USDC.e |

### Order Matching & Settlement (EIP-712)

Signed order fields:
- `maker` -- order creator address
- `taker` -- counterparty (zero address = public)
- `tokenId` -- the outcome token
- `makerAmount` / `takerAmount` -- quantities
- `expiration`, `nonce`, `feeRateBps`, `salt`
- `signature` -- EIP-712 signature

Three matching scenarios:
1. **NORMAL** -- direct token-for-token or token-for-collateral swap
2. **MINT** -- both parties buy complementary outcomes; system mints token sets from combined collateral
3. **MERGE** -- both parties sell complementary outcomes; system merges tokens into collateral

### Fee Structure

Fees are symmetric across complementary pairs:
- `feeQuote = baseRate * min(price, 1-price) * size`
- Example: At $0.50 price with 2% base rate, buying 100 shares = $1.00 fee

### Resolution

- Markets are resolved by the **UMA Optimistic Oracle**
- The UMA CTF Adapter (`0x6A9D222616C90FcA5754cd1333cFD9b7fb6a4F74`) reports outcomes on-chain
- After resolution, winning outcome tokens can be redeemed 1:1 for USDC.e
- Losing outcome tokens become worthless

### Collateral

- **USDC.e** (bridged USDC) on Polygon: `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` (6 decimals)

### Token Allowances (Required for EOA/MetaMask Wallets)

Before trading, EOA wallets must approve these contracts to spend USDC.e and Conditional Tokens:

**USDC.e** (`0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174`) must be approved for:
- CTF Exchange: `0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E`
- Neg Risk CTF Exchange: `0xC5d563A36AE78145C45a50134d48A1215220f80a`
- Neg Risk Adapter: `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296`

**Conditional Tokens** (`0x4D97DCd97eC945f40cF65F87097ACe5EA0476045`) must be approved for the same three contracts.

Email/Magic wallets have allowances set automatically.

---

## 9. Rate Limits

All rate limits are enforced via Cloudflare. When exceeded, requests are **throttled** (delayed/queued), not immediately rejected. Limits use sliding time windows.

### CLOB API (`https://clob.polymarket.com`)

| Endpoint | Limit |
|----------|-------|
| **General** | 9,000 req / 10s |
| `/book`, `/price`, `/midpoint` (single) | 1,500 req / 10s |
| `/books`, `/prices`, `/midpoints` (batch) | 500 req / 10s |
| `/prices-history` | 1,000 req / 10s |
| `/tick-size` | 200 req / 10s |
| `/balance-allowance` GET | 200 req / 10s |
| `/balance-allowance/update` | 50 req / 10s |
| Auth endpoints | 100 req / 10s |
| Trades/orders/notifications | 900 req / 10s |
| `/data/orders`, `/data/trades` | 500 req / 10s |
| `/notifications` | 125 req / 10s |

**Trading endpoints (dual limits -- burst + sustained):**

| Endpoint | Burst (10s) | Sustained (10min) |
|----------|-------------|-------------------|
| `POST /order` | 3,500 | 36,000 |
| `DELETE /order` | 3,000 | 30,000 |
| `POST /orders` (batch) | 1,000 | 15,000 |
| `DELETE /orders` (batch) | 1,000 | 15,000 |
| `DELETE /cancel-all` | 250 | 6,000 |
| `DELETE /cancel-market-orders` | 1,000 | 1,500 |

### Gamma API (`https://gamma-api.polymarket.com`)

| Endpoint | Limit |
|----------|-------|
| General | 4,000 req / 10s |
| `/events` | 500 req / 10s |
| `/markets` | 300 req / 10s |
| `/markets` + `/events` combined | 900 req / 10s |
| `/comments` | 200 req / 10s |
| `/tags` | 200 req / 10s |
| `/public-search` | 350 req / 10s |

### Data API (`https://data-api.polymarket.com`)

| Endpoint | Limit |
|----------|-------|
| General | 1,000 req / 10s |
| `/trades` | 200 req / 10s |
| `/positions` | 150 req / 10s |
| `/closed-positions` | 150 req / 10s |

### Other
- Health check (`/ok`): 100 req / 10s
- Relayer `/submit`: 25 req / 1min

---

## 10. On-Chain Contracts & Copy Trading

### Contract Addresses (Polygon Mainnet, Chain ID 137)

| Contract | Address | Purpose |
|----------|---------|---------|
| **CTF Exchange** | `0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E` | Standard binary market order matching & settlement |
| **Neg Risk CTF Exchange** | `0xC5d563A36AE78145C45a50134d48A1215220f80a` | Multi-outcome market order matching |
| **Neg Risk Adapter** | `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296` | Token conversion for neg risk markets |
| **Conditional Tokens (CTF)** | `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` | ERC1155 token operations |
| **USDC.e** | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` | Collateral token (6 decimals) |
| **UMA CTF Adapter** | `0x6A9D222616C90FcA5754cd1333cFD9b7fb6a4F74` | Oracle integration for resolution |
| **UMA Optimistic Oracle** | `0xCB1822859cEF82Cd2Eb4E6276C7916e692995130` | Dispute & resolution system |
| **Gnosis Safe Factory** | `0xaacfeea03eb1561c4e67d661e40682bd20e3541b` | Safe wallet deployment |
| **Polymarket Proxy Factory** | `0xaB45c5A4B0c941a2F231C04C3f49182e1A254052` | Proxy wallet deployment |
| **Uniswap v3 USDC.e/USDC Pool** | `0xd36ec33c8bed5a9f7b6630855f1533455b98a418` | Token conversion during withdrawals |
| **Neg Risk Fee Module** | `0x78769d50be1763ed1ca0d5e878d93f05aabff29e` | Fee handling for neg risk |

### On-Chain Event Signatures for Wallet Tracking

| Event | Signature Hash | Emitted By |
|-------|---------------|------------|
| **OrderFilled** | `0xd0a08e8c493f9c94f29311604c9de1b4e8c8d4c06bd0c789af57f2d65bfec0f6` | CTF Exchange, Neg Risk CTF Exchange |
| **OrdersMatched** | `0x63bf4d16b7fa898ef4c4b2b6d90fd201e9c56313b65638af6088d149d2ce956c` | CTF Exchange, Neg Risk CTF Exchange |
| **PositionSplit** | `0xbbed930dbfb7907ae2d60ddf78345610214f26419a0128df39b6cc3d9e5df9b0` | CTF, NegRiskAdapter |
| **PositionsMerge** | `0xba33ac50d8894676597e6e35dc09cff59854708b642cd069d21eb9c7ca072a04` | CTF, NegRiskAdapter |
| **PositionsConverted** | `0xb03d19dddbc72a87e735ff0ea3b57bef133ebe44e1894284916a84044deb367e` | NegRiskAdapter |

### Copy Trading Implementation Approach

**Dual-layer detection:**

1. **REST API Polling (Discovery Layer):**
   - Query `GET https://data-api.polymarket.com/activity?user={target_wallet}&type=TRADE`
   - Filter for new trades by timestamp
   - Provides: side, size, price, tokenId, timestamp

2. **On-Chain Monitoring (Real-time Layer):**
   - Connect to Polygon RPC endpoint
   - Subscribe to `OrderFilled` events from CTF Exchange / Neg Risk CTF Exchange
   - Filter by target wallet address in maker/taker fields
   - Decode `makerAssetId`: 0 = USDC (buy order), non-zero = outcome token (sell order)
   - Note: taker can be the NegRisk_CTFExchange address when aggregating orders

3. **WebSocket Monitoring:**
   - Subscribe to market channel for price/book updates on markets the target trades
   - Use user channel (if you have their credentials -- only for your own account)

**Trade lifecycle for copy bot:**
detect target trade -> risk check -> size calculation -> sign order via CLOB -> submit order -> track fill

---

## 11. SDK Clients & GitHub Repos

### Official Polymarket Repositories

| Repo | Language | URL |
|------|----------|-----|
| **py-clob-client** | Python | `https://github.com/Polymarket/py-clob-client` |
| **clob-client** | TypeScript | `https://github.com/Polymarket/clob-client` |
| **ctf-exchange** | Solidity | `https://github.com/Polymarket/ctf-exchange` |
| **neg-risk-ctf-adapter** | Solidity | `https://github.com/Polymarket/neg-risk-ctf-adapter` |
| **conditional-token-examples** | TypeScript | `https://github.com/Polymarket/conditional-token-examples` |
| **agents** | Python | `https://github.com/Polymarket/agents` |

### Package Installation

```bash
# Python
pip install py-clob-client

# TypeScript/JavaScript
npm install @polymarket/clob-client

# Rust
cargo add polymarket-client-sdk
```

### Key Source Files to Study

- **Endpoints**: `py_clob_client/endpoints.py` -- all REST path constants
- **Types**: `py_clob_client/clob_types.py` -- OrderArgs, MarketOrderArgs, OrderType, etc.
- **Auth Headers**: `py_clob_client/headers/headers.py` -- L1/L2 header construction
- **HMAC Signing**: `py_clob_client/signing/hmac.py` -- HMAC-SHA256 implementation
- **EIP-712 Signing**: `py_clob_client/signing/eip712.py` -- wallet signature for auth
- **Contract Config**: `py_clob_client/config.py` -- all contract addresses per chain
- **Client**: `py_clob_client/client.py` -- full API client (~1082 lines)

### Documentation Index
- Full docs: `https://docs.polymarket.com/`
- LLM-friendly index: `https://docs.polymarket.com/llms.txt`

---

## Quick Reference: Trading Bot Adapter Checklist

1. **Initialize client** with private key, chain_id=137, signature_type, and funder address
2. **Derive API credentials** via `create_or_derive_api_creds()` (L1 auth -> L2 creds)
3. **Discover markets** via Gamma API (`GET /events?active=true&closed=false`)
4. **Get token IDs** from market's `tokens` array (Yes and No token IDs)
5. **Fetch order book** via `GET /book?token_id={id}`
6. **Subscribe WebSocket** for real-time updates on market channel
7. **Create signed orders** via `create_order()` or `create_market_order()`
8. **Submit orders** via `POST /order` with L2 auth headers
9. **Monitor fills** via user WebSocket channel or poll `GET /data/trades`
10. **Manage positions** via Data API `GET /positions?user={address}`

### Important Gotchas
- Prices are always 0.00 to 1.00 (representing probability/dollar value)
- Token IDs are very long integers (ERC1155 position IDs)
- Markets must have `enable_order_book=true` to trade via CLOB
- EOA wallets need token approvals before first trade
- Heartbeats: once started, must continue every 10s or all orders auto-cancel
- Batch limit: max 15 orders per `POST /orders` call
- Neg risk markets use different exchange contract
- Tick sizes change dynamically near extreme prices (>0.96 or <0.04)
