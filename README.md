-## What is this project?
This project serves as Rust learning materials.

## Support commands
1. [Generic commands](https://redis.io/commands/?group=generic)
- [X] DEL
- [x] EXPIRE
- [x] TTL
- [x] EXISTS
- [x] FLUSH

2. [String commands](https://redis.io/commands/?group=string)
- [x] GET
- [x] SET
- [x] SETNX
- [x] MGET
- [x] MSET
- [x] GETSET
- [x] GETDEL
- [x] INCR
- [x] DECR
- [x] INCRBY
- [x] DECRBY

3. [List commands](https://redis.io/commands/?group=list)
- [x] LPOP
- [x] RPOP
- [x] LPUSH
- [x] RPUSH
- [ ] LLEN
- [ ] LRANGE
- [ ] LREM
- [ ] LSET
- [ ] LTRIM
- [ ] BLPOP
- [ ] BRPOP

4. [Hash commands](https://redis.io/commands/?group=hash)
- [ ] HLEN
- [ ] HKEYS
- [ ] HGET
- [ ] HMGET
- [ ] HGETALL
- [ ] HINCRBY
- [ ] HSET
- [ ] HMSET
- [ ] HSETNX

## Benchmark

We use redis-benchmark (shipped with Redis) as our go to benchmark tool. To benchmark, simply run:
```shell
bash script/bench.sh
```

The test suite simulates 100 clients, each makes 100000 requests. Under the hood, we run this:

```shell
redis-benchmark -h localhost -p <port> -c 100 -n 100000 -k 1 -t <commands> --csv
```

| Command | redis (op/s) | tiny_redis (op/s) | Comparison |
| --- | --- | --- | --- |
| GET | 89445.44 | 58038.3 | ❌ -35.11% |
| SET | 87260.03 | 54614.96 | ❌ -37.41% |
| INCR | 88888.89 | 57870.37 | ❌ -34.90% |
| LPOP | 80710.25 | 59772.86 | ❌ -25.94% |
| RPOP | 86430.43 | 52576.24 | ❌ -39.17% |
| LPUSH | 93896.71 | 61012.81 | ❌ -35.02% |
| RPUSH | 91157.7 | 62656.64 | ❌ -31.27% |