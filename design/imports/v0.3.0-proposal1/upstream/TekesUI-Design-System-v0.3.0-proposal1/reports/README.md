# 报告范围

proposal-validation、brand-validation与context-validation是本包可复现静态校验。negative-tests包含负向拦截和品牌正向/边界测试，不全部是负向测试。expected-*.json是待执行矩阵，不是通过记录。

release-readiness的fail是预期的真实阻断：artifact、批准、原生组件、真实应用/环境证据缺失。所有品牌数值检查以生成8bit sRGB值进行；未验证渲染像素与人眼状态辨识。

report-integrity校验报告文件字节，不证明它们的结论或来源；源文件摘要在manifest.json中。
