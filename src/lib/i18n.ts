import type { SupportedLanguage } from "./settings";

export const DEFAULT_LANGUAGE: SupportedLanguage = "zh-CN";

type TranslationKey =
  | "app.localMonitor"
  | "common.online"
  | "common.waiting"
  | "common.notSupported"
  | "common.enabled"
  | "common.disabled"
  | "common.confirmClear"
  | "nav.overview"
  | "nav.history"
  | "nav.settings"
  | "device.loading"
  | "overview.currentSnapshot"
  | "overview.sensorDetails"
  | "overview.waitingFirstSample"
  | "sensors.temperature"
  | "sensors.voltage"
  | "sensors.current"
  | "sensors.power"
  | "sensors.energy"
  | "sensors.fan"
  | "metrics.cpu"
  | "metrics.memory"
  | "metrics.disk"
  | "metrics.network"
  | "metrics.gpu"
  | "metrics.temperature"
  | "metrics.power"
  | "metrics.battery"
  | "metrics.memoryUsed"
  | "metrics.diskUsed"
  | "metrics.diskRead"
  | "metrics.diskWrite"
  | "metrics.gpuName"
  | "metrics.gpuMemoryTotal"
  | "metrics.gpuUsage"
  | "metrics.temperatureCelsius"
  | "metrics.powerWatts"
  | "metrics.download"
  | "metrics.upload"
  | "history.refresh"
  | "history.device"
  | "history.emptyTitle"
  | "history.emptyBody"
  | "history.cpuSeries"
  | "history.memoryDisk"
  | "history.memorySeries"
  | "history.memoryBytes"
  | "history.memoryBytesSeries"
  | "history.diskSeries"
  | "history.diskSpeed"
  | "history.networkSeries"
  | "history.gpuSeries"
  | "history.temperatureSeries"
  | "history.powerSeries"
  | "settings.sampling"
  | "settings.machineName"
  | "settings.sampleInterval"
  | "settings.saveInterval"
  | "settings.language"
  | "settings.metrics"
  | "settings.s3.title"
  | "settings.s3.enabled"
  | "settings.s3.endpoint"
  | "settings.s3.region"
  | "settings.s3.bucket"
  | "settings.s3.prefix"
  | "settings.s3.accessKey"
  | "settings.s3.secretKey"
  | "settings.s3.syncInterval"
  | "settings.s3.pathStyle"
  | "settings.s3.test"
  | "settings.s3.syncNow"
  | "settings.s3.testOk"
  | "settings.s3.syncOk"
  | "settings.unsupportedHint"
  | "settings.save"
  | "settings.saved"
  | "settings.localData.title"
  | "settings.localData.databaseSize"
  | "settings.localData.sampleCount"
  | "settings.localData.databasePath"
  | "settings.localData.clearHistory"
  | "settings.localData.cleared";

const translations: Record<SupportedLanguage, Record<TranslationKey, string>> = {
  "zh-CN": {
    "app.localMonitor": "本地监控",
    "common.online": "在线",
    "common.waiting": "等待中",
    "common.notSupported": "未支持",
    "common.enabled": "已开启",
    "common.disabled": "已关闭",
    "common.confirmClear": "确认清理本地历史采样数据？设置会保留。",
    "nav.overview": "概览",
    "nav.history": "历史",
    "nav.settings": "设置",
    "device.loading": "正在加载设备信息",
    "overview.currentSnapshot": "当前快照",
    "overview.sensorDetails": "传感器明细",
    "overview.waitingFirstSample": "等待首次采样",
    "sensors.temperature": "温度",
    "sensors.voltage": "电压",
    "sensors.current": "电流",
    "sensors.power": "功率",
    "sensors.energy": "能耗",
    "sensors.fan": "风扇",
    "metrics.cpu": "CPU",
    "metrics.memory": "内存",
    "metrics.disk": "磁盘",
    "metrics.network": "网络",
    "metrics.gpu": "GPU",
    "metrics.temperature": "温度",
    "metrics.power": "功率",
    "metrics.battery": "电池",
    "metrics.memoryUsed": "已用内存",
    "metrics.diskUsed": "已用磁盘",
    "metrics.diskRead": "磁盘读取",
    "metrics.diskWrite": "磁盘写入",
    "metrics.gpuName": "显卡",
    "metrics.gpuMemoryTotal": "显存",
    "metrics.gpuUsage": "GPU 使用率",
    "metrics.temperatureCelsius": "当前温度",
    "metrics.powerWatts": "当前功率",
    "metrics.download": "下载",
    "metrics.upload": "上传",
    "history.refresh": "刷新",
    "history.device": "设备",
    "history.emptyTitle": "暂无历史数据",
    "history.emptyBody": "保持应用运行，等待本地采样写入。",
    "history.cpuSeries": "CPU %",
    "history.memoryDisk": "内存 / 磁盘",
    "history.memorySeries": "内存 %",
    "history.memoryBytes": "内存用量",
    "history.memoryBytesSeries": "已用内存",
    "history.diskSeries": "磁盘 %",
    "history.diskSpeed": "磁盘速度",
    "history.networkSeries": "网络",
    "history.gpuSeries": "GPU",
    "history.temperatureSeries": "温度",
    "history.powerSeries": "功率",
    "settings.sampling": "采样设置",
    "settings.machineName": "本机名称",
    "settings.sampleInterval": "采样间隔（秒）",
    "settings.saveInterval": "本地保存间隔（秒）",
    "settings.language": "语言",
    "settings.metrics": "指标开关",
    "settings.s3.title": "S3 同步",
    "settings.s3.enabled": "启用 S3 同步",
    "settings.s3.endpoint": "S3 地址",
    "settings.s3.region": "Region",
    "settings.s3.bucket": "Bucket",
    "settings.s3.prefix": "目录",
    "settings.s3.accessKey": "Access Key",
    "settings.s3.secretKey": "Secret Key",
    "settings.s3.syncInterval": "同步间隔（分钟）",
    "settings.s3.pathStyle": "Path-style 地址",
    "settings.s3.test": "测试连接",
    "settings.s3.syncNow": "立即同步",
    "settings.s3.testOk": "S3 连接成功",
    "settings.s3.syncOk": "S3 同步完成：上传 {uploadedDays} 天，拉取 {downloadedDevices} 台设备，导入 {importedSamples} 条采样",
    "settings.unsupportedHint": "当前版本只保留开关，采集稍后支持。",
    "settings.save": "保存",
    "settings.saved": "设置已保存",
    "settings.localData.title": "本地数据",
    "settings.localData.databaseSize": "数据库大小",
    "settings.localData.sampleCount": "历史采样数量",
    "settings.localData.databasePath": "数据库路径",
    "settings.localData.clearHistory": "清理历史数据",
    "settings.localData.cleared": "历史数据已清理",
  },
  en: {
    "app.localMonitor": "Local Monitor",
    "common.online": "Online",
    "common.waiting": "Waiting",
    "common.notSupported": "Not supported",
    "common.enabled": "Enabled",
    "common.disabled": "Disabled",
    "common.confirmClear": "Clear local history samples? Settings will be kept.",
    "nav.overview": "Overview",
    "nav.history": "History",
    "nav.settings": "Settings",
    "device.loading": "Loading device profile",
    "overview.currentSnapshot": "Current Snapshot",
    "overview.sensorDetails": "Sensor Details",
    "overview.waitingFirstSample": "Waiting for first sample",
    "sensors.temperature": "Temperature",
    "sensors.voltage": "Voltage",
    "sensors.current": "Current",
    "sensors.power": "Power",
    "sensors.energy": "Energy",
    "sensors.fan": "Fan",
    "metrics.cpu": "CPU",
    "metrics.memory": "Memory",
    "metrics.disk": "Disk",
    "metrics.network": "Network",
    "metrics.gpu": "GPU",
    "metrics.temperature": "Temperature",
    "metrics.power": "Power",
    "metrics.battery": "Battery",
    "metrics.memoryUsed": "Memory Used",
    "metrics.diskUsed": "Disk Used",
    "metrics.diskRead": "Disk Read",
    "metrics.diskWrite": "Disk Write",
    "metrics.gpuName": "GPU",
    "metrics.gpuMemoryTotal": "GPU Memory",
    "metrics.gpuUsage": "GPU Usage",
    "metrics.temperatureCelsius": "Temperature",
    "metrics.powerWatts": "Power",
    "metrics.download": "Download",
    "metrics.upload": "Upload",
    "history.refresh": "Refresh",
    "history.device": "Device",
    "history.emptyTitle": "No history yet",
    "history.emptyBody": "Keep the app running until local samples are saved.",
    "history.cpuSeries": "CPU %",
    "history.memoryDisk": "Memory / Disk",
    "history.memorySeries": "Memory %",
    "history.memoryBytes": "Memory Usage",
    "history.memoryBytesSeries": "Memory Used",
    "history.diskSeries": "Disk %",
    "history.diskSpeed": "Disk Speed",
    "history.networkSeries": "Network",
    "history.gpuSeries": "GPU",
    "history.temperatureSeries": "Temperature",
    "history.powerSeries": "Power",
    "settings.sampling": "Sampling",
    "settings.machineName": "Machine name",
    "settings.sampleInterval": "Sample interval (seconds)",
    "settings.saveInterval": "Local save interval (seconds)",
    "settings.language": "Language",
    "settings.metrics": "Metric toggles",
    "settings.s3.title": "S3 Sync",
    "settings.s3.enabled": "Enable S3 sync",
    "settings.s3.endpoint": "S3 endpoint",
    "settings.s3.region": "Region",
    "settings.s3.bucket": "Bucket",
    "settings.s3.prefix": "Prefix",
    "settings.s3.accessKey": "Access Key",
    "settings.s3.secretKey": "Secret Key",
    "settings.s3.syncInterval": "Sync interval (minutes)",
    "settings.s3.pathStyle": "Path-style URL",
    "settings.s3.test": "Test connection",
    "settings.s3.syncNow": "Sync now",
    "settings.s3.testOk": "S3 connection succeeded",
    "settings.s3.syncOk": "S3 sync complete: uploaded {uploadedDays} days, downloaded {downloadedDevices} devices, imported {importedSamples} samples",
    "settings.unsupportedHint": "Toggle is saved now; collection will be added later.",
    "settings.save": "Save",
    "settings.saved": "Settings saved",
    "settings.localData.title": "Local data",
    "settings.localData.databaseSize": "Database size",
    "settings.localData.sampleCount": "History samples",
    "settings.localData.databasePath": "Database path",
    "settings.localData.clearHistory": "Clear history data",
    "settings.localData.cleared": "History data cleared",
  },
};

export function t(key: TranslationKey, language: SupportedLanguage): string {
  return translations[language][key] ?? translations[DEFAULT_LANGUAGE][key];
}
