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

export async function addUser(body: {
  user_id: string
  platform: string
  username?: string
  first_name?: string
  state?: string
}) {
  return apiFetch('/users', { method: 'POST', body: JSON.stringify(body) })
}

export async function makeAdmin(platform: string, userId: string) {
  return apiFetch(`/users/${platform}/${userId}/make-admin`, { method: 'PUT' })
}

export async function removeAdmin(platform: string, userId: string) {
  return apiFetch(`/users/${platform}/${userId}/remove-admin`, { method: 'PUT' })
}

export async function getUserStats() {
  return apiFetch('/stats')
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
