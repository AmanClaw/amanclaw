import "clsx";
import { e as ensure_array_like, a as attr_class, s as stringify, b as store_get, c as escape_html, u as unsubscribe_stores } from "../../chunks/index2.js";
import { c as currentPage } from "../../chunks/app.js";
function Sidebar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    const pages = [
      { id: "dashboard", label: "Dashboard", icon: "⊞" },
      { id: "communities", label: "Communities", icon: "⊡" },
      { id: "skills", label: "Skills", icon: "⚡" },
      { id: "users", label: "Users", icon: "⊙" },
      { id: "content", label: "Content", icon: "☰" },
      { id: "logs", label: "Logs", icon: "▤" }
    ];
    const bottomPages = [{ id: "settings", label: "Settings", icon: "⚙" }];
    $$renderer2.push(`<aside class="w-56 h-screen bg-gray-50/80 backdrop-blur-xl border-r border-gray-200 flex flex-col justify-between p-3"><div><div class="px-3 py-4 mb-2"><h1 class="text-sm font-semibold text-gray-900 tracking-tight">AmanClaw</h1></div> <nav class="space-y-0.5"><!--[-->`);
    const each_array = ensure_array_like(pages);
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let page = each_array[$$index];
      $$renderer2.push(`<button${attr_class(`w-full flex items-center gap-2.5 px-3 py-1.5 rounded-md text-[13px] transition-colors ${stringify(store_get($$store_subs ??= {}, "$currentPage", currentPage) === page.id ? "bg-gray-200/80 text-gray-900 font-medium" : "text-gray-600 hover:bg-gray-100 hover:text-gray-900")}`)}><span class="text-base leading-none">${escape_html(page.icon)}</span> ${escape_html(page.label)}</button>`);
    }
    $$renderer2.push(`<!--]--></nav></div> <div><div class="border-t border-gray-200 pt-2 mb-2"><!--[-->`);
    const each_array_1 = ensure_array_like(bottomPages);
    for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
      let page = each_array_1[$$index_1];
      $$renderer2.push(`<button class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-md text-[13px] text-gray-600 hover:bg-gray-100 hover:text-gray-900 transition-colors"><span class="text-base leading-none">${escape_html(page.icon)}</span> ${escape_html(page.label)}</button>`);
    }
    $$renderer2.push(`<!--]--></div> <div class="mx-2 p-2.5 bg-white rounded-lg border border-gray-200 shadow-sm"><div class="flex items-center gap-2"><span class="w-2 h-2 rounded-full bg-green-500"></span> <span class="text-[11px] font-medium text-gray-700">Bot Running</span></div></div></div></aside>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function _layout($$renderer, $$props) {
  let { children } = $$props;
  $$renderer.push(`<div class="flex h-screen bg-white select-none">`);
  Sidebar($$renderer);
  $$renderer.push(`<!----> <main class="flex-1 overflow-y-auto">`);
  children($$renderer);
  $$renderer.push(`<!----></main></div>`);
}
export {
  _layout as default
};
