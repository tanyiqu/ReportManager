

```
pnpm install

pnpm tauri dev
```

## 清理构建产物

在项目根目录双击或在命令行执行 `clean.bat`，可删除可重新生成的前端构建目录、Tauri/Rust 编译产物、Tauri 能力定义缓存、Vite 缓存、覆盖率目录和项目日志。

该脚本不会删除 `node_modules`、源代码、配置文件或应用本地数据；需要重新安装依赖时请自行执行 `pnpm install`。
