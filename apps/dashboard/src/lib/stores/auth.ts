import { writable } from 'svelte/store'

export const isLoggedIn = writable(true) // TODO: restore to false when backend auth is wired up
