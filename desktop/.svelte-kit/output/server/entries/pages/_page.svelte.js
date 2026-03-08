import { a as attr_class, aa as attr, c as escape_html, s as stringify, e as ensure_array_like, a9 as derived, b as store_get, u as unsubscribe_stores } from "../../chunks/index2.js";
import "@tauri-apps/api/core";
import { c as currentPage, b as botStatus } from "../../chunks/app.js";
import "clsx";
function Communities($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    $$renderer2.push(`<div class="p-8 max-w-4xl"><div class="flex items-center justify-between mb-8"><div><h2 class="text-xl font-semibold text-gray-900 tracking-tight">Communities</h2> <p class="text-sm text-gray-500 mt-1">Manage your connected groups</p></div> <button class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">Add Community</button></div> `);
    {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="text-sm text-gray-500">Loading...</p>`);
    }
    $$renderer2.push(`<!--]--></div>`);
  });
}
function Skills($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    $$renderer2.push(`<div class="p-8 max-w-4xl"><div class="mb-8"><h2 class="text-xl font-semibold text-gray-900 tracking-tight">Skills</h2> <p class="text-sm text-gray-500 mt-1">Manage bot capabilities</p></div> `);
    {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="text-sm text-gray-500">Loading...</p>`);
    }
    $$renderer2.push(`<!--]--></div>`);
  });
}
function Users($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    $$renderer2.push(`<div class="p-8 max-w-4xl"><div class="mb-8"><h2 class="text-xl font-semibold text-gray-900 tracking-tight">Users</h2> <p class="text-sm text-gray-500 mt-1">Manage bot users and permissions</p></div> `);
    {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="text-sm text-gray-500">Loading...</p>`);
    }
    $$renderer2.push(`<!--]--></div>`);
  });
}
function Settings($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let mode = "local";
    $$renderer2.push(`<div class="p-8 max-w-2xl"><div class="mb-8"><h2 class="text-xl font-semibold text-gray-900 tracking-tight">Settings</h2> <p class="text-sm text-gray-500 mt-1">Configure your AmanClaw instance</p></div> <div class="mb-8"><h3 class="text-sm font-medium text-gray-900 mb-3">Connection Mode</h3> <div class="space-y-2"><label${attr_class(`flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors ${stringify("border-gray-900 bg-gray-50")}`)}><input type="radio"${attr("checked", mode === "local", true)} value="local" class="accent-gray-900"/> <div><p class="text-sm font-medium text-gray-900">Local Mode</p> <p class="text-xs text-gray-500">Bot runs on this machine</p></div></label> <label${attr_class(`flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors ${stringify("")}`)}><input type="radio"${attr("checked", mode === "remote", true)} value="remote" class="accent-gray-900"/> <div><p class="text-sm font-medium text-gray-900">Remote Mode</p> <p class="text-xs text-gray-500">Connect to a remote AmanClaw server</p></div></label></div> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <button class="mt-4 px-4 py-2 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">${escape_html("Save")}</button></div> <div class="border-t border-gray-200 pt-6"><h3 class="text-sm font-medium text-gray-900 mb-2">About</h3> <p class="text-xs text-gray-500">AmanClaw Desktop v0.1.0</p> <p class="text-xs text-gray-500">Built in Malaysia</p></div></div>`);
  });
}
function Logs($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let logs = [];
    let filter = "";
    let filteredLogs = derived(() => logs);
    $$renderer2.push(`<div class="p-8 max-w-5xl"><div class="flex items-center justify-between mb-6"><div><h2 class="text-xl font-semibold text-gray-900 tracking-tight">Logs</h2> <p class="text-sm text-gray-500 mt-1">Live bot activity</p></div> <input type="text"${attr("value", filter)} placeholder="Filter logs..." class="px-3 py-1.5 text-xs border border-gray-200 rounded-md w-48 focus:outline-none focus:ring-2 focus:ring-gray-900"/></div> <div class="bg-gray-950 rounded-xl p-4 font-mono text-xs h-[calc(100vh-200px)] overflow-y-auto"><!--[-->`);
    const each_array = ensure_array_like(filteredLogs());
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let log = each_array[$$index];
      $$renderer2.push(`<div class="py-0.5 flex gap-3"><span class="text-gray-600 shrink-0">${escape_html(log.timestamp)}</span> <span${attr_class(`shrink-0 ${stringify(log.level === "ERROR" ? "text-red-400" : log.level === "WARN" ? "text-yellow-400" : log.level === "INFO" ? "text-blue-400" : "text-gray-500")}`)}>${escape_html(log.level)}</span> <span class="text-gray-300">${escape_html(log.message)}</span></div>`);
    }
    $$renderer2.push(`<!--]--> `);
    if (filteredLogs().length === 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="text-gray-600">No logs yet. Start the bot to see activity.</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div></div>`);
  });
}
function Content($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let activeTab = "doa";
    $$renderer2.push(`<div class="p-8 max-w-4xl"><div class="mb-6"><h2 class="text-xl font-semibold text-gray-900 tracking-tight">Content</h2> <p class="text-sm text-gray-500 mt-1">Manage Islamic content and data</p></div> <div class="flex gap-1 mb-6 bg-gray-100 p-1 rounded-lg w-fit"><!--[-->`);
    const each_array = ensure_array_like(["doa", "zakat", "khutbah"]);
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let tab = each_array[$$index];
      $$renderer2.push(`<button${attr_class(`px-3 py-1.5 text-xs font-medium rounded-md transition-colors ${stringify(activeTab === tab ? "bg-white text-gray-900 shadow-sm" : "text-gray-600 hover:text-gray-900")}`)}>${escape_html(tab.charAt(0).toUpperCase() + tab.slice(1))}</button>`);
    }
    $$renderer2.push(`<!--]--></div> `);
    {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="bg-gray-50 rounded-xl border border-gray-200 p-5"><div class="flex items-center justify-between mb-4"><p class="text-sm font-medium text-gray-900">Doa Collection</p> <button class="text-xs text-gray-500 hover:text-gray-900">Add Doa</button></div> <p class="text-xs text-gray-500">20 doas across 9 categories. Edit via the collection manager.</p></div>`);
    }
    $$renderer2.push(`<!--]--></div>`);
  });
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    if (store_get($$store_subs ??= {}, "$currentPage", currentPage) === "communities") {
      $$renderer2.push("<!--[0-->");
      Communities($$renderer2);
    } else if (store_get($$store_subs ??= {}, "$currentPage", currentPage) === "skills") {
      $$renderer2.push("<!--[1-->");
      Skills($$renderer2);
    } else if (store_get($$store_subs ??= {}, "$currentPage", currentPage) === "users") {
      $$renderer2.push("<!--[2-->");
      Users($$renderer2);
    } else if (store_get($$store_subs ??= {}, "$currentPage", currentPage) === "settings") {
      $$renderer2.push("<!--[3-->");
      Settings($$renderer2);
    } else if (store_get($$store_subs ??= {}, "$currentPage", currentPage) === "logs") {
      $$renderer2.push("<!--[4-->");
      Logs($$renderer2);
    } else if (store_get($$store_subs ??= {}, "$currentPage", currentPage) === "content") {
      $$renderer2.push("<!--[5-->");
      Content($$renderer2);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="p-8 max-w-4xl"><div class="mb-8"><h2 class="text-xl font-semibold text-gray-900 tracking-tight">Dashboard</h2> <p class="text-sm text-gray-500 mt-1">Overview of your AmanClaw instance</p></div> <div class="grid grid-cols-3 gap-4 mb-8"><div class="bg-gray-50 rounded-xl border border-gray-200 p-5"><p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Communities</p> <p class="text-2xl font-semibold text-gray-900 mt-1">${escape_html(store_get($$store_subs ??= {}, "$botStatus", botStatus).communities)}</p></div> <div class="bg-gray-50 rounded-xl border border-gray-200 p-5"><p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Active Skills</p> <p class="text-2xl font-semibold text-gray-900 mt-1">${escape_html(store_get($$store_subs ??= {}, "$botStatus", botStatus).skills)}</p></div> <div class="bg-gray-50 rounded-xl border border-gray-200 p-5"><p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Users</p> <p class="text-2xl font-semibold text-gray-900 mt-1">${escape_html(store_get($$store_subs ??= {}, "$botStatus", botStatus).users)}</p></div></div> <div class="bg-gray-50 rounded-xl border border-gray-200 p-5"><div class="flex items-center justify-between"><div class="flex items-center gap-3"><span${attr_class(`w-3 h-3 rounded-full ${stringify(store_get($$store_subs ??= {}, "$botStatus", botStatus).running ? "bg-green-500" : "bg-red-500")}`)}></span> <div><p class="text-sm font-medium text-gray-900">${escape_html(store_get($$store_subs ??= {}, "$botStatus", botStatus).running ? "Bot Running" : "Bot Stopped")}</p> <p class="text-xs text-gray-500">${escape_html(store_get($$store_subs ??= {}, "$botStatus", botStatus).mode === "local" ? "Local Mode" : "Remote Mode")}</p></div></div> <button class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 text-gray-700 hover:bg-gray-100 transition-colors">${escape_html(store_get($$store_subs ??= {}, "$botStatus", botStatus).running ? "Stop" : "Start")}</button></div></div></div>`);
    }
    $$renderer2.push(`<!--]-->`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
