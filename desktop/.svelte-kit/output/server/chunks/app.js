import { w as writable } from "./index.js";
const botStatus = writable({
  running: false,
  mode: "local",
  communities: 0,
  users: 0,
  skills: 0
});
const currentPage = writable("dashboard");
export {
  botStatus as b,
  currentPage as c
};
