import { FormEvent, PointerEvent as ReactPointerEvent, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Page = string;
type NavigationMenu = { id: string; label: string; iconSvg: string; sortOrder: number; isSystem: boolean };
type Preferences = { sidebarCollapsed: boolean; defaultPageId: string; weekStart: string; exportDirectory: string; minimizeToTray: boolean; menus: NavigationMenu[] };
type Dialog = "manage" | "add" | "rename" | "icon" | "delete" | null;
type DropPlacement = "before" | "after";
type MenuDropTarget = { id: string; placement: DropPlacement };
const SVG_NAMESPACE = "http://www.w3.org/2000/svg";
const makeIcon = (paths: string) => `<svg xmlns="${SVG_NAMESPACE}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${paths}</svg>`;
const iconChoices = [
  { name: "首页", svg: makeIcon('<path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1Z"/><path d="M9 21v-6h6v6"/>') },
  { name: "日报", svg: makeIcon('<path d="M6 3h9l3 3v15H6z"/><path d="M9 12h6M9 16h6M9 8h3"/>') },
  { name: "文件夹", svg: makeIcon('<path d="M3 7h7l2 2h9v11H3z"/>') },
  { name: "清单", svg: makeIcon('<path d="M9 6h11M9 12h11M9 18h11"/><path d="m3 6 1 1 2-2m-3 7 1 1 2-2m-3 7 1 1 2-2"/>') },
  { name: "设置", svg: makeIcon('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.1 2.1-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1.03 1.55V20.3h-3v-.1A1.7 1.7 0 0 0 10.7 18.65a1.7 1.7 0 0 0-1.88.34l-.06.06-2.1-2.1.06-.06A1.7 1.7 0 0 0 7.06 15 1.7 1.7 0 0 0 5.5 14H5.4v-3h.1a1.7 1.7 0 0 0 1.56-1.03 1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.1-2.1.06.06a1.7 1.7 0 0 0 1.88.34A1.7 1.7 0 0 0 11.7 4.8v-.1h3v.1a1.7 1.7 0 0 0 1.03 1.55 1.7 1.7 0 0 0 1.88-.34l.06-.06 2.1 2.1-.06.06a1.7 1.7 0 0 0-.34 1.88A1.7 1.7 0 0 0 20.9 11h.1v3h-.1A1.7 1.7 0 0 0 19.4 15Z"/>') },
];
const toolbarIcons = { menu: makeIcon('<path d="M4 6h16M4 12h16M4 18h16"/>'), search: makeIcon('<circle cx="11" cy="11" r="6"/><path d="m20 20-4-4"/>') };
const menuActionIcons = {
  rename: makeIcon('<path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L8 18l-4 1 1-4Z"/>'),
  changeIcon: makeIcon('<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-5-5L5 21"/>'),
  delete: makeIcon('<path d="M3 6h18M8 6V4h8v2M19 6l-1 15H6L5 6M10 11v5M14 11v5"/>'),
};
const today = new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "long", day: "numeric", weekday: "long" }).format(new Date());

function moveMenu(menus: NavigationMenu[], sourceId: string, target: MenuDropTarget): NavigationMenu[] | null {
  const source = menus.find((menu) => menu.id === sourceId);
  const destination = menus.find((menu) => menu.id === target.id);
  if (!source || !destination || source.isSystem || destination.isSystem || source.id === destination.id) return null;

  const reordered = menus.filter((menu) => menu.id !== sourceId);
  const targetIndex = reordered.findIndex((menu) => menu.id === target.id);
  if (targetIndex < 0) return null;
  // Resolve the insertion point after removing the source so before/after
  // placement stays correct for both upward and downward moves.
  reordered.splice(targetIndex + (target.placement === "after" ? 1 : 0), 0, source);

  if (reordered.every((menu, index) => menu.id === menus[index]?.id)) return null;
  return reordered.map((menu, sortOrder) => ({ ...menu, sortOrder }));
}

function SvgIcon({ svg }: { svg: string }) { const standaloneSvg = /<svg\b[^>]*\bxmlns=/.test(svg) ? svg : svg.replace(/<svg\b/, `<svg xmlns="${SVG_NAMESPACE}"`); const bytes = new TextEncoder().encode(standaloneSvg); let binary = ""; for (const byte of bytes) binary += String.fromCharCode(byte); const image = `url("data:image/svg+xml;base64,${btoa(binary)}")`; return <span className="svg-icon" aria-hidden="true" style={{ WebkitMaskImage: image, maskImage: image }} />; }
function Toasts({ messages, onDismiss }: { messages: { id: number; message: string }[]; onDismiss: (id: number) => void }) { return <div className="toast-stack" aria-live="polite">{messages.map((toast) => <button className="toast" key={toast.id} role="status" title="点击关闭提示" onClick={() => onDismiss(toast.id)}>{toast.message}</button>)}</div>; }

export default function App() {
  const [preferences, setPreferences] = useState<Preferences | null>(null); const [page, setPage] = useState<Page>("home"); const [editing, setEditing] = useState(false); const [title, setTitle] = useState(`${today.replace(/星期.*/, "")}日报`); const [content, setContent] = useState(""); const [notice, setNotice] = useState(""); const [toasts, setToasts] = useState<{ id: number; message: string }[]>([]);
  const menu = preferences?.menus.find((item) => item.id === page); const label = menu?.label ?? "首页";
  const showToast = (message: string) => { const id = Date.now() + Math.random(); setToasts((current) => [...current, { id, message }]); window.setTimeout(() => setToasts((current) => current.filter((toast) => toast.id !== id)), 3200); };
  useEffect(() => { void invoke("show_main_window").catch((error: unknown) => console.error("无法显示主窗口", error)); void invoke<Preferences>("get_app_preferences").then((saved) => { setPreferences(saved); setPage(saved.menus.some((item) => item.id === saved.defaultPageId) ? saved.defaultPageId : "home"); }).catch((error: unknown) => console.error("无法读取偏好设置", error)); }, []);
  const persist = (next: Preferences, successMessage = "已保存设置") => { setPreferences(next); void invoke<Preferences>("save_app_preferences", { preferences: next }).then((saved) => { setPreferences(saved); showToast(successMessage); }).catch((error: unknown) => { console.error("无法保存偏好设置", error); showToast("设置保存失败，请稍后重试。"); }); };
  const applySaved = (next: Preferences, message = "已保存设置") => { setPreferences(next); showToast(message); };
  const createDaily = () => { setPage("daily"); setEditing(true); setNotice(""); };
  if (!preferences) return <div className="app-loading">正在载入本地设置…</div>;
  return <div className={`app-shell ${preferences.sidebarCollapsed ? "sidebar-collapsed" : ""}`}><aside className="sidebar"><div className="sidebar-toolbar"><span className="brand-name">ReportManager</span><div className="toolbar-actions"><button className="toolbar-icon" title={preferences.sidebarCollapsed ? "展开侧边栏" : "收起侧边栏"} aria-label={preferences.sidebarCollapsed ? "展开侧边栏" : "收起侧边栏"} onClick={() => persist({ ...preferences, sidebarCollapsed: !preferences.sidebarCollapsed })}><SvgIcon svg={toolbarIcons.menu} /></button><button className="toolbar-icon search-button" title="搜索（即将推出）" aria-label="搜索（即将推出）" onClick={() => showToast("搜索功能即将推出")}><SvgIcon svg={toolbarIcons.search} /></button></div></div><nav aria-label="主导航">{preferences.menus.map((item) => <button key={item.id} title={preferences.sidebarCollapsed ? item.label : undefined} className={`nav-item ${page === item.id ? "active" : ""}`} onClick={() => { setPage(item.id); setEditing(false); }}><SvgIcon svg={item.iconSvg} /><span className="nav-label">{item.label}</span></button>)}</nav><div className="sidebar-bottom"><span className="offline-dot" /><span className="offline-text">所有内容仅保存在本机</span></div></aside><main className="main-content"><header className="topbar"><div><p className="eyebrow">{label}</p><h1>{page === "home" ? "工作记录，一目了然" : label}</h1></div><button className="primary" onClick={createDaily}>＋ 新建今日日报</button></header>{page === "home" && <Home onCreate={createDaily} />}{page === "daily" && (editing ? <DailyEditor title={title} content={content} notice={notice} onTitle={setTitle} onContent={setContent} onSave={(event) => { event.preventDefault(); setNotice("草稿已保存。连接本地数据库后将自动持久化到 SQLite。"); }} /> : <RecordList type={label} onCreate={createDaily} />)}{(page === "weekly" || page === "meeting" || (!menu?.isSystem && page !== "home")) && <RecordList type={label} onCreate={() => setEditing(true)} />}{page === "settings" && <Settings preferences={preferences} onPersist={persist} onApplySaved={applySaved} onNavigate={setPage} />}</main><Toasts messages={toasts} onDismiss={(id) => setToasts((current) => current.filter((toast) => toast.id !== id))} /></div>;
}

function Home({ onCreate }: { onCreate: () => void }) { return <div className="page-stack"><section className="welcome-card"><div><p className="muted">今天是 {today}</p><h2>从一份清晰的日报开始</h2><p>记录进展、风险与下一步计划，让工作脉络随时可追溯。</p><button className="primary" onClick={onCreate}>创建今日日报</button></div><span className="calendar">20</span></section><section className="status-grid">{[["今日日报", "尚未创建", "立即创建"], ["本周周报", "等待整理", "查看周报"], ["今日例会", "尚未记录", "开始记录"]].map(([itemLabel, value, action]) => <article className="status-card" key={itemLabel}><p>{itemLabel}</p><h3>{value}</h3><button className="text-button">{action} →</button></article>)}</section><section className="panel"><div className="section-heading"><div><h2>最近编辑</h2><p>你最近更新的工作记录会显示在这里。</p></div><button className="text-button">查看全部记录 →</button></div><div className="empty-state"><span>▤</span><h3>还没有工作记录</h3><p>创建第一份日报，开始沉淀你的工作成果。</p></div></section></div>; }
function RecordList({ type, onCreate }: { type: string; onCreate: () => void }) { return <div className="page-stack"><section className="toolbar panel"><input aria-label="搜索记录" placeholder={`搜索${type}标题、正文或标签`} /><button className="secondary">筛选日期</button><button className="secondary">标签</button><button className="primary" onClick={onCreate}>新建{type}</button></section><section className="panel empty-state"><span>⌕</span><h3>暂无{type}</h3><p>创建后可以按日期、标签和关键词快速检索。</p></section></div>; }
function DailyEditor({ title, content, notice, onTitle, onContent, onSave }: { title: string; content: string; notice: string; onTitle: (v: string) => void; onContent: (v: string) => void; onSave: (event: FormEvent) => void }) { return <form className="editor page-stack" onSubmit={onSave}><section className="editor-header"><div><input className="title-input" value={title} onChange={(e) => onTitle(e.target.value)} aria-label="日报标题" /><p className="muted">记录日期：{today}</p></div><div className="editor-actions"><button type="button" className="secondary">复制纯文本</button><button type="button" className="secondary">导出 Markdown</button><button className="primary">保存草稿</button></div></section>{notice && <p className="notice" role="status">{notice}</p>}<section className="editor-grid"><div className="panel writing"><label htmlFor="daily-content">正文</label><textarea id="daily-content" value={content} onChange={(e) => onContent(e.target.value)} placeholder={"1. 今日完成：\n2. 进行中：\n3. 问题与风险：\n4. 下一步计划："} /></div><aside className="panel structured-fields"><h2>结构化信息（可选）</h2>{["完成事项", "进行中事项", "问题与风险", "下一步计划", "标签"].map((field) => <label key={field}>{field}<input placeholder={`填写${field}`} /></label>)}</aside></section></form>; }
function DialogShell({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) { return <div className="dialog-backdrop" role="presentation" onMouseDown={onClose}><section className="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title" onMouseDown={(event) => event.stopPropagation()}><div className="dialog-heading"><h2 id="dialog-title">{title}</h2><button className="dialog-close" onClick={onClose} aria-label="关闭对话框">×</button></div>{children}</section></div>; }

function Settings({ preferences, onPersist, onApplySaved, onNavigate }: { preferences: Preferences; onPersist: (next: Preferences, message?: string) => void; onApplySaved: (next: Preferences, message?: string) => void; onNavigate: (page: Page) => void }) {
  const [dialog, setDialog] = useState<Dialog>(null); const [newLabel, setNewLabel] = useState(""); const [newIcon, setNewIcon] = useState(iconChoices[1].svg); const [selectedMenu, setSelectedMenu] = useState<NavigationMenu | null>(null); const [draftLabel, setDraftLabel] = useState(""); const [draftIcon, setDraftIcon] = useState(""); const [draggedId, setDraggedId] = useState<string | null>(null); const [dropTarget, setDropTarget] = useState<MenuDropTarget | null>(null);
  // Pointer capture keeps move/up events attached to the handle in WebView2,
  // while refs make the latest drag state available before React re-renders.
  const draggedIdRef = useRef<string | null>(null); const dropTargetRef = useRef<MenuDropTarget | null>(null);
  const openEditor = (menu: NavigationMenu, mode: "rename" | "icon") => { setSelectedMenu(menu); setDraftLabel(menu.label); setDraftIcon(menu.iconSvg); setDialog(mode); };
  const saveMenu = (changes: Partial<NavigationMenu>, message: string) => { if (!selectedMenu) return; onPersist({ ...preferences, menus: preferences.menus.map((menu) => menu.id === selectedMenu.id ? { ...menu, ...changes } : menu) }, message); setDialog("manage"); };
  const addMenu = () => { const label = newLabel.trim(); if (!label) return; const menu: NavigationMenu = { id: `custom-${crypto.randomUUID()}`, label, iconSvg: newIcon, sortOrder: preferences.menus.length - 1, isSystem: false }; void invoke<Preferences>("create_navigation_menu", { menu }).then((next) => { onApplySaved(next, "菜单已添加"); setNewLabel(""); setDialog("manage"); }).catch((error: unknown) => window.alert(String(error))); };
  const remove = () => { if (!selectedMenu || selectedMenu.isSystem) return; void invoke<Preferences>("delete_navigation_menu", { id: selectedMenu.id }).then((next) => { if (preferences.defaultPageId === selectedMenu.id) onPersist({ ...next, defaultPageId: "home" }, "菜单已删除"); else onApplySaved(next, "菜单已删除"); onNavigate("settings"); setDialog("manage"); }).catch((error: unknown) => window.alert(String(error))); };
  const clearDropTarget = () => { dropTargetRef.current = null; setDropTarget(null); };
  const resetDrag = () => { draggedIdRef.current = null; clearDropTarget(); setDraggedId(null); };
  const updateDropTarget = (event: ReactPointerEvent<HTMLButtonElement>) => {
    const row = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>("[data-menu-id]");
    const targetId = row?.dataset.menuId;
    const target = preferences.menus.find((menu) => menu.id === targetId);
    if (!row || !target || target.isSystem || target.id === draggedIdRef.current) {
      clearDropTarget();
      return;
    }
    const bounds = row.getBoundingClientRect();
    const nextTarget: MenuDropTarget = { id: target.id, placement: event.clientY < bounds.top + bounds.height / 2 ? "before" : "after" };
    dropTargetRef.current = nextTarget;
    setDropTarget(nextTarget);
  };
  const finishDrag = () => {
    const sourceId = draggedIdRef.current;
    const target = dropTargetRef.current;
    resetDrag();
    if (!sourceId || !target) return;
    const menus = moveMenu(preferences.menus, sourceId, target);
    if (menus) onPersist({ ...preferences, menus }, "菜单顺序已更新");
  };
  const startDrag = (event: ReactPointerEvent<HTMLButtonElement>, item: NavigationMenu) => {
    if (item.isSystem || !event.isPrimary || event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    draggedIdRef.current = item.id;
    setDraggedId(item.id);
  };
  return <div className="page-stack settings-page"><section className="panel settings"><h2>偏好设置</h2><label>启动后默认进入页面<select value={preferences.defaultPageId} onChange={(event) => onPersist({ ...preferences, defaultPageId: event.target.value })}>{preferences.menus.map((item) => <option key={item.id} value={item.id}>{item.label}</option>)}</select></label><label>界面语言<select defaultValue="zh-CN"><option value="zh-CN">简体中文</option></select></label><label>每周起始日<select value={preferences.weekStart} onChange={(event) => onPersist({ ...preferences, weekStart: event.target.value })}><option value="monday">周一</option><option value="sunday">周日</option></select></label><label>默认导出目录<input value={preferences.exportDirectory} placeholder="尚未设置，导出时选择" onChange={(event) => onPersist({ ...preferences, exportDirectory: event.target.value })} /></label></section><section className="panel menu-settings"><div className="section-heading"><div><h2>菜单管理</h2><p>调整导航菜单的顺序、名称和图标；首页始终置顶，设置始终置底。</p></div><button className="secondary" onClick={() => setDialog("manage")}>菜单管理</button></div></section><section className="panel settings-other"><h2>其他</h2><label className="checkbox-setting"><input type="checkbox" checked={preferences.minimizeToTray} onChange={(event) => onPersist({ ...preferences, minimizeToTray: event.target.checked }, "关闭窗口行为已更新")} /><span><strong>关闭窗口时最小化到系统托盘</strong><small>开启后，点击关闭按钮会隐藏主窗口；关闭后会直接退出程序。</small></span></label></section><section className="panel setting-row"><div><h3>本地数据备份</h3><p>导出全部数据以便备份或迁移。内容不会上传到网络。</p></div><button className="secondary">导出全部数据</button></section>
    {dialog === "manage" && <DialogShell title="菜单管理" onClose={() => { resetDrag(); setDialog(null); }}><p className="dialog-description">拖动自定义菜单左侧的手柄即可调整顺序，松开后会立即保存。</p><div className="menu-list" onPointerLeave={clearDropTarget}>{preferences.menus.map((item) => { const dropClass = dropTarget?.id === item.id ? `is-drop-${dropTarget.placement}` : ""; return <div className={`menu-row ${draggedId === item.id ? "is-dragging" : ""} ${dropClass}`} data-menu-id={item.id} key={item.id}><button type="button" className="drag-handle" disabled={item.isSystem} title={item.isSystem ? "系统菜单不可移动" : "拖动调整顺序"} aria-label={item.isSystem ? "系统菜单不可移动" : `拖动“${item.label}”调整顺序`} onPointerDown={(event) => startDrag(event, item)} onPointerMove={updateDropTarget} onPointerUp={finishDrag} onPointerCancel={resetDrag} onPointerLeave={clearDropTarget}>⠿</button><SvgIcon svg={item.iconSvg} /><span className="menu-row-label">{item.label}</span><div className="menu-actions"><button className="icon-button" title="重命名菜单" aria-label="重命名菜单" onClick={() => openEditor(item, "rename")}><SvgIcon svg={menuActionIcons.rename} /></button><button className="icon-button" title="修改菜单图标" aria-label="修改菜单图标" onClick={() => openEditor(item, "icon")}><SvgIcon svg={menuActionIcons.changeIcon} /></button><button className="icon-button danger-icon" title={item.isSystem ? "首页与设置不可删除" : "删除菜单"} aria-label={item.isSystem ? "首页与设置不可删除" : "删除菜单"} disabled={item.isSystem} {...(!item.isSystem ? { onClick: () => { setSelectedMenu(item); setDialog("delete"); } } : {})}><SvgIcon svg={menuActionIcons.delete} /></button></div></div>; })}</div><button className="secondary add-menu-button" onClick={() => setDialog("add")}>＋ 添加菜单</button></DialogShell>}
    {dialog === "add" && <DialogShell title="添加菜单" onClose={() => setDialog("manage")}><p className="dialog-description">新菜单会添加在“设置”菜单之前。</p><label className="dialog-field">菜单名称<input value={newLabel} onChange={(event) => setNewLabel(event.target.value)} placeholder="例如：客户项目" autoFocus /></label><label className="dialog-field">菜单图标<select value={newIcon} onChange={(event) => setNewIcon(event.target.value)}>{iconChoices.map((choice) => <option key={choice.name} value={choice.svg}>{choice.name}</option>)}</select></label><div className="dialog-actions"><button className="secondary" onClick={() => setDialog("manage")}>取消</button><button className="primary" disabled={!newLabel.trim()} onClick={addMenu}>添加菜单</button></div></DialogShell>}
    {dialog === "rename" && selectedMenu && <DialogShell title="重命名菜单" onClose={() => setDialog("manage")}><label className="dialog-field">新名称<input value={draftLabel} onChange={(event) => setDraftLabel(event.target.value)} autoFocus /></label><div className="dialog-actions"><button className="secondary" onClick={() => setDialog("manage")}>取消</button><button className="primary" disabled={!draftLabel.trim()} onClick={() => saveMenu({ label: draftLabel.trim() }, "菜单名称已更新")}>保存</button></div></DialogShell>}
    {dialog === "icon" && selectedMenu && <DialogShell title="修改菜单图标" onClose={() => setDialog("manage")}><label className="dialog-field">菜单图标<select value={draftIcon} onChange={(event) => setDraftIcon(event.target.value)} autoFocus>{iconChoices.map((choice) => <option key={choice.name} value={choice.svg}>{choice.name}</option>)}</select></label><div className="dialog-actions"><button className="secondary" onClick={() => setDialog("manage")}>取消</button><button className="primary" onClick={() => saveMenu({ iconSvg: draftIcon }, "菜单图标已更新")}>保存</button></div></DialogShell>}
    {dialog === "delete" && selectedMenu && <DialogShell title="删除菜单" onClose={() => setDialog("manage")}><p className="dialog-description">确定删除“{selectedMenu.label}”吗？仅可删除没有关联报告的自定义菜单，删除后不可恢复。</p><div className="dialog-actions"><button className="secondary" onClick={() => setDialog("manage")}>取消</button><button className="danger-button" onClick={remove}>确认删除</button></div></DialogShell>}</div>;
}
