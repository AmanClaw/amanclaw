const BASE = '/api'

export async function apiFetch(path: string, opts: RequestInit = {}) {
  const res = await fetch(`${BASE}${path}`, {
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json', ...(opts.headers as Record<string, string>) },
    ...opts,
  })
  if (res.status === 401) {
    window.location.hash = '#/login'
    throw new Error('Unauthorized')
  }
  return res.json()
}

export async function login(password: string) {
  const res = await fetch(`${BASE}/login`, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password }),
  })
  if (!res.ok) throw new Error('Invalid password')
  return res.json()
}
