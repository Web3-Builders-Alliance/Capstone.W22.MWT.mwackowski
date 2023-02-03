# ETS creation on COSMOS ecosystem's chains
The project consists of 3 contracts:
- manager
- swap (for osmosis)
- cw20 (minting tokens)

The project allows user to instantiate multiple ETFs that are represented as separate cw20 contracts. Based on the ETF instantiated user is able to swap his tokens based on provided structure (denoms/shares), in exchange a proof of purchased ETF is minted via cw20 contract. User can also redeem tokens, which burns cw20-proof tokens and swaps and send all of the initially received tokens.
The project will be extended so that it will allow staking of the tokens and reinvesting/donating rewards.
Eventually, the project will also include other DEXes/chains.



