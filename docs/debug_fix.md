# GitHub Push认证失败问题排查与解决方案

## 问题描述

在尝试推送本地commits到GitHub远程仓库时，遇到持续的认证失败问题：
- 多个有效的GitHub Personal Access Token (PAT) 都无法通过git push认证
- API调用正常工作，但git操作失败
- 错误信息：`remote: invalid credentials`

## 排查过程

### 第一阶段：验证Token权限

1. **API权限测试**
```bash
# 测试token是否有效
curl -H "Authorization: token ghp_xxx" https://api.github.com/user

# 检查仓库权限
curl -H "Authorization: token ghp_xxx" https://api.github.com/repos/USER/REPO | grep permissions
# 结果显示: "push": true ✅
```

2. **API写入测试**（关键对照实验）
```bash
# 获取main分支SHA
curl -s -H "Authorization: token ghp_xxx" \
  https://api.github.com/repos/USER/REPO/git/ref/heads/main | jq -r '.object.sha'

# 创建测试分支
curl -X POST -H "Authorization: token ghp_xxx" \
  https://api.github.com/repos/USER/REPO/git/refs \
  -d '{"ref":"refs/heads/api-test-branch","sha":"SHA_HERE"}'
# 结果: 201 Created ✅

# 删除测试分支
curl -X DELETE -H "Authorization: token ghp_xxx" \
  https://api.github.com/repos/USER/REPO/git/refs/heads/api-test-branch
```

**结论**：Token权限正常，问题在git客户端认证链路。

### 第二阶段：Git认证链路排查

3. **查看Git认证详情**
```bash
GIT_TRACE=1 GIT_CURL_VERBOSE=1 git push
# 发现: Authorization: bearer <redacted> ❌
# 应该是: Basic authentication
```

4. **检查extraheader配置**（找到根因！）
```bash
# 查找所有extraheader配置
git config --global --get-regexp '^http\..*\.extraheader'
git config --system --get-regexp '^http\..*\.extraheader'
git config --local --get-regexp '^http\..*\.extraheader'

# 发现问题：
# http.https://github.com/.extraheader AUTHORIZATION: bearer ghp_xxx
```

5. **清理错误配置**
```bash
# 删除所有extraheader配置
git config --local --unset-all http.https://github.com/.extraheader
git config --global --unset-all http.https://github.com/.extraheader
```

### 第三阶段：处理GitHub Push Protection

6. **推送时遇到新问题**
```
remote: error: GH013: Repository rule violations found
remote: - GITHUB PUSH PROTECTION
remote: Push cannot contain secrets
```

7. **清理敏感信息**
```bash
# 查找包含token的文件
git ls-files | xargs grep -l "ghp_"

# 替换所有token
sed -i 's/ghp_[A-Za-z0-9_]*/YOUR_GITHUB_TOKEN_HERE/g' file1 file2 ...
```

8. **重写历史移除secrets**
```bash
# 创建干净的分支
git checkout -b clean-branch <last-clean-commit>

# Cherry-pick每个commit并清理
for commit in commit1 commit2 ...; do
  git cherry-pick -n $commit
  # 清理文件中的token
  sed -i 's/ghp_[A-Za-z0-9_]*/YOUR_TOKEN_HERE/g' $(git diff --cached --name-only)
  git commit -m "$(git log -1 --format=%B $commit)"
done
```

## 最终解决方案

### 问题根因

1. **http.extraheader配置污染**
   - 本地git仓库存在错误的`http.https://github.com/.extraheader`配置
   - 导致Git发送`Authorization: bearer`而非Basic认证
   - 这通常是GitHub Actions或某些脚本残留的配置

2. **GitHub Push Protection**
   - 历史commits中包含暴露的GitHub tokens
   - GitHub的secret scanning阻止了包含敏感信息的推送

### 修复步骤

```bash
# 1. 清理extraheader配置
git config --local --unset-all http.https://github.com/.extraheader
git config --global --unset-all http.https://github.com/.extraheader

# 2. 测试干净推送
git -c credential.helper= push https://github.com/USER/REPO.git HEAD:branch-name
# 输入用户名和PAT

# 3. 如果有secret scanning问题，重写历史
git checkout -b clean-branch <last-clean-commit>
# Cherry-pick并清理每个commit
# 推送清理后的分支

# 4. 强制更新原分支（如需要）
git push origin clean-branch:original-branch --force
```

## 排查思路总结

### 诊断流程图

```
认证失败
    ↓
1. 验证Token权限
    ├─ API调用测试 → ✅ 成功
    └─ API写入测试 → ✅ 成功
        ↓
    Token权限正常，问题在git客户端
        ↓
2. 检查Git认证链路
    ├─ 查看GIT_TRACE输出
    ├─ 检查credential.helper
    └─ 检查http.extraheader ← 找到问题！
        ↓
3. 清理配置并重试
    ├─ 成功 → 完成
    └─ 失败 → 检查其他问题
        ↓
4. 处理GitHub Push Protection
    └─ 清理历史中的secrets
```

### 关键诊断命令

1. **权限验证**
```bash
# API权限测试
curl -H "Authorization: token TOKEN" https://api.github.com/repos/USER/REPO | jq .permissions

# 创建分支测试（最佳对照实验）
curl -X POST -H "Authorization: token TOKEN" \
  https://api.github.com/repos/USER/REPO/git/refs \
  -d '{"ref":"refs/heads/test","sha":"COMMIT_SHA"}'
```

2. **认证链路调试**
```bash
# 详细追踪
GIT_TRACE=1 GIT_CURL_VERBOSE=1 git -c credential.helper= push

# 检查所有git配置
git config --list --show-origin | grep -E "credential|http"
```

3. **配置清理**
```bash
# 查找并清理extraheader
git config --get-regexp '^http\..*\.extraheader'
git config --unset-all http.https://github.com/.extraheader
```

## 预防措施

1. **定期检查git配置**
```bash
# 添加到.bashrc或定期运行
alias git-check-auth='git config --get-regexp "extraheader|credential"'
```

2. **避免在代码中硬编码tokens**
   - 使用环境变量
   - 使用GitHub Secrets
   - 使用credential managers

3. **使用GitHub CLI作为备选**
```bash
# 安装gh并设置
gh auth login
gh auth setup-git
```

## 常见陷阱

1. **extraheader污染**
   - 来源：GitHub Actions、CI/CD脚本
   - 症状：API正常但git push失败
   - 解决：清理所有extraheader配置

2. **credential.helper缓存**
   - 旧的或错误的凭据被缓存
   - 解决：`git config --global credential.helper cache --timeout=0`

3. **Fine-grained PAT权限**
   - 需要明确选择仓库和权限
   - Contents: Read and Write
   - Metadata: Read

4. **网络/防火墙问题**
   - SSH端口22被封锁
   - 解决：使用HTTPS或SSH over 443端口

## 相关链接

- [GitHub Token Authentication](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token)
- [GitHub Push Protection](https://docs.github.com/en/code-security/secret-scanning/working-with-secret-scanning-and-push-protection)
- [Git Credential Storage](https://git-scm.com/docs/git-credential)
- [GitHub CLI](https://cli.github.com/)

## 总结

这个问题的核心在于git配置被污染，导致错误的认证方式。通过系统性的排查：
1. 先验证权限（API测试）
2. 再查认证链路（GIT_TRACE）
3. 最后清理配置（extraheader）

记住：**如果API能创建/删除分支但git push失败，几乎肯定是客户端认证配置问题。**