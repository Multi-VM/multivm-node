import { deserialize } from "borsh";
import { Buffer } from "buffer";

const OrderSide = {
    enum: [
      {
        struct: {
          ask: {
            struct: {}
          }
        }
      },
      {
        struct: {
          bid: {
            struct: {}
        }
      }
    }
  ]
};

const OrderStatus = {
  enum: [
    {
      struct: {
        new: {
          struct: {}
        }
      }
    },
    {
      struct: {
        cancelled: {
          struct: {}
        }
      }
    },
    {
      struct: {
        filled: {
          struct: {}
        }
      }
    },
    {
      struct: {
        partially_filled: {
          struct: {}
        }
      }
    },
    {
      struct: {
        limit_reached: {
          struct: {}
        }
      }
    }
  ]
};

const BalanceChangeInfo = {
  enum: [
    {
      struct: {
        external: {
          struct: {}
        }
      }
    },
    {
      struct: {
        locked: 'u128'
      }
    },
    {
      struct: {
        trade: 'u128'
      }
    }
  ]
};

const Event = {
  enum: [
    {
      struct: {
        orderbook: {
          struct: {
            market_id: 'u16',
            price: 'u128',
            quantity: 'u128',
            side: OrderSide
          }
        }
      }
    },
    {
      struct: {
        order: {
          struct: {
            market_id: 'u16',
            order_id: 'u128',
            user_id: 'string',
            price: 'u128',
            average_price: 'u128',
            quantity: 'u128',
            remaining: 'u128',
            side: OrderSide,
            status: OrderStatus,
            client_order_id: {
              option: 'u32'
            }
          }
        }
      }
    },
    {
      struct: {
        trade: {
          struct: {
            market_id: 'u16',
            maker_order_id: 'u128',
            taker_order_id: 'u128',
            maker_fee: 'u128',
            maker_fee_token_id: 'u16',
            taker_fee: 'u128',
            taker_fee_token_id: 'u16',
            is_maker_fee_rebate: 'bool',
            price: 'u128',
            quantity: 'u128',
            side: OrderSide
          }
        }
      }
    },
    {
      struct: {
        balance: {
          struct: {
            user_id: 'string',
            token_id: 'u16',
            amount: 'u128',
            increased: 'bool',
            change_type: BalanceChangeInfo
          }
        }
      }
    }
  ]
};

const EventWithMeta = {
  struct: {
    data: Event,
    id: 'u64',
  },
};

const LogsSchema = {
  array: { type: EventWithMeta },
};
const log = "CQAAAAABAFoAAAAAAAAAAAAAAAAAAAAAAICw0eCMXOrCqAAAAAAAAcBYAAAAAAAAA0AAAAA4MTE3YjY2MTIxMGNiOTI5MThmOTE2NjMxZmQ2MzY1MjhjYTMwMWZjNTZlMjBkNWJlOWMzMTZkNmY4MzNmYmQ1AQAAAABC25mdN4SnAQAAAAAAAAFhCgAAAAAAAAAAAAAAAAAAwVgAAAAAAAACAQDjAAAAAAAAAAAAAAAAAAAAYQoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAAAAAAAAAAAAAAAAAAAAAACAAFaAAAAAAAAAAAAAAAAAAAAAAAAQtuZnTeEpwEAAAAAAADCWAAAAAAAAAMOAAAAbWluYTg4LnRlc3RuZXQCALQAAAAAAAAAAAAAAAAAAAAAAuMAAAAAAAAAAAAAAAAAAADDWAAAAAAAAAMOAAAAbWluYTg4LnRlc3RuZXQBAAAAAELbmZ03hKcBAAAAAAABAuMAAAAAAAAAAAAAAAAAAADEWAAAAAAAAAEBAOMAAAAAAAAAAAAAAAAAAAAOAAAAbWluYTg4LnRlc3RuZXRaAAAAAAAAAAAAAAAAAAAAWgAAAAAAAAAAAAAAAAAAAAAAgBzCclSa6+xcAAAAAAAAAMDiYHOSBcR6JQAAAAAAAQMAxVgAAAAAAAADQAAAADgxMTdiNjYxMjEwY2I5MjkxOGY5MTY2MzFmZDYzNjUyOGNhMzAxZmM1NmUyMGQ1YmU5YzMxNmQ2ZjgzM2ZiZDUBAAAAAELbmZ03hKcBAAAAAAAAAmEKAAAAAAAAAAAAAAAAAADGWAAAAAAAAANAAAAAODExN2I2NjEyMTBjYjkyOTE4ZjkxNjYzMWZkNjM2NTI4Y2EzMDFmYzU2ZTIwZDViZTljMzE2ZDZmODMzZmJkNQIAswAAAAAAAAAAAAAAAAAAAAECYQoAAAAAAAAAAAAAAAAAAMdYAAAAAAAAAQEAYQoAAAAAAAAAAAAAAAAAAEAAAAA4MTE3YjY2MTIxMGNiOTI5MThmOTE2NjMxZmQ2MzY1MjhjYTMwMWZjNTZlMjBkNWJlOWMzMTZkNmY4MzNmYmQ1WgAAAAAAAAAAAAAAAAAAAFoAAAAAAAAAAAAAAAAAAAAAAABC25mdN4SnAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAACATkwAADIWAAAAAAAAA==";

const decode = (str: string):string => Buffer.from(str, 'base64').toString('binary');
const decoded = decode(log);

const logs = deserialize(LogsSchema, Buffer.from(decoded, "binary"));
console.log("========", logs);
console.log("========", logs[0].data);

