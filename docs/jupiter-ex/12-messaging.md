# Сообщения, спавн, скрипты команд

## Спавн (**SDK**)

```
DEdit props → BEGIN_CLASS / LINKTO_MODULE (ltserverobj.h)
  → IObjectPlugin (ReadProps)
  → ObjectCreateStruct (ltobjectcreate.h)
  → ILTServer::CreateObject  или  ILTClient::CreateObject
  → ILTBaseClass колбэки (OnSave/OnLoad, touch, update)
```

`CreateObject` живёт в exe (`CLTServer`/`CLTClient`); retail CF и маппинг полей OCS → runtime `+0x57`/`+0x58`/`+0x9c` — **unknown**. Post-create layout: [27-visibility-sort.md](27-visibility-sort.md).

Сообщения: `ILTCommon::CreateMessage` → `ILTMessage_Write` → `ILTServer::SendToObject` / `SendToServer` → `ObjectMessageFn` / `IServerShell::OnObjectMessage`.

`EngineMessageFn` — старый путь, ещё жив у `WorldProperties`. (**SDK**)

## CommandMgr (сервер)

Розничный `GameServer.dll` содержит `CCommandMgr::Process*` и `CAICommandMgr::Handle*Msg`. Это **уровневые команды** (скрипты объектов), не Lua. Клиент имеет `ScmdConsole*` (админ-консоль). (**строки DLL**)

Движок: `CLTClient/Server::RegisterConsoleProgram`, `Cvar_Set`. (**Ghidra**)

Имена сообщений ИИ на сервере: `AddGoal` / `RemoveGoal` / `GoalSet` (ошибки MSG). Это вход в [15-ai.md](15-ai.md).

## Prefetch

`IObjectResourceGatherer` собирает модели / WorldModel / регионы до загрузки. `ILTResourceMgr` держит lifetime. (**SDK**)
