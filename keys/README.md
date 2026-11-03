# JWT keys

Generate a unique key pair locally before starting the server:

```sh
mkdir -p keys
(umask 077; openssl genpkey -algorithm RSA -out keys/private.pem -pkeyopt rsa_keygen_bits:2048)
openssl pkey -in keys/private.pem -pubout -out keys/public.pem
```

Never commit generated keys. Replacing a leaked signing key requires updating the server and invalidating existing sessions.
