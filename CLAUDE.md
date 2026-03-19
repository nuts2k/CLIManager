# Claude Code project instructions

## cc-switch directory is read-only

`cc-switch/` 目录仅做参考代码；不要对它及其子目录以及目录下的文件做任何增加、修改、删除操作。

`cc-switch/` 目录下全是参考代码，非当前项目代码

## cc-switch 仅供参考，不受其局限

在做方案调研或各种 research 工作时，可以将 `cc-switch/` 目录下的原有实现方式当做参考，但仅限参考，不要受它局限。

## 发版规范

本项目发版统一使用 `/ship` 技能，禁止自动触发 `release` 技能。

无论用户说"发版"、"升级版本"、"发布新版本"、"bump version"等，均应调用 `/ship`，而不是 `release`。

## 语言规范

在对话沟通、文档、编码注释中，一律尽量使用中文