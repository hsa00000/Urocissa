import { getSrc } from '@/../config.ts'

export function getSrcWithToken(
  hash: string,
  original: boolean,
  ext: string,
  _password: string,
  _customParams: unknown,
  token: string
) {
  const url = getSrc(hash, original, ext, _password, _customParams)
  const urlWithToken = `${url}?token=${token}`
  console.log('urlWithToken is', urlWithToken)

  return urlWithToken
}
