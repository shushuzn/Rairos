# Rairos MVP 用户反馈收集

## 背景

Rairos 正在开发 **Paper-Code-Dataset 三位一体跨模态研究管道** MVP。

## 测试功能

### 1. GitHub 仓库元数据查询
```json
{
  "name": "github_repo_metadata",
  "params": {
    "owner": "pytorch",
    "repo": "pytorch",
    "include_readme": "true"
  }
}
```

返回：stars, forks, language, license, topics, README preview

### 2. HuggingFace 数据集查询
```json
{
  "name": "huggingface_dataset_metadata",
  "params": {
    "dataset_id": "imagenet-1k"
  }
}
```

或搜索模式：
```json
{
  "name": "huggingface_dataset_metadata",
  "params": {
    "search": "image classification",
    "limit": "5"
  }
}
```

## 反馈问题

### Q1: 功能有用性
这些功能对你有帮助吗？
- [ ] 非常有用
- [ ] 有一定帮助
- [ ] 不太有用
- [ ] 完全没用

### Q2: 使用场景
你在什么场景下会使用这些功能？
- 论文筛选
- 复现验证
- 研究 Gap 检测
- 其他：________

### Q3: 缺失信息
你认为还应该包含哪些信息？
- 代码活跃度（最后更新时间）
- 依赖环境要求
- benchmark 结果
- 其他：________

### Q4: 付费意愿
如果这些功能作为付费服务，你愿意支付多少？
- 愿意免费使用
- $5/月
- $10/月
- $20/月
- 愿意付费但不超过 $__/月

### Q5: 改进建议
请提供任何改进建议：________

## 联系方式

如果你愿意提供反馈，请联系：________

---

感谢你的反馈！
