"""一键发布 — POST /api/deploy 推送到 Vercel / Netlify。"""

import re
import os
import subprocess
import asyncio
from pathlib import Path
from fastapi import APIRouter, Request
from pydantic import BaseModel

router = APIRouter()


class DeployRequest(BaseModel):
    projectDir: str          # 项目目录绝对路径
    platform: str = "vercel" # "vercel" | "netlify"
    token: str = ""          # 平台 token（可选，也可用环境变量）


class DeployResponse(BaseModel):
    success: bool
    url: str = ""
    platform: str = ""
    error: str = ""


@router.post("/deploy")
async def deploy(req: DeployRequest, request: Request):
    """推送项目到 Vercel 或 Netlify。"""
    project_dir = Path(req.projectDir)

    if not project_dir.exists():
        return DeployResponse(
            success=False,
            error=f"项目目录不存在: {req.projectDir}",
        ).model_dump()

    if req.platform == "vercel":
        return await _deploy_vercel(project_dir, req.token)
    elif req.platform == "netlify":
        return await _deploy_netlify(project_dir, req.token)
    else:
        return DeployResponse(
            success=False,
            error=f"不支持的平台: {req.platform}。可选: vercel, netlify",
        ).model_dump()


async def _deploy_vercel(project_dir: Path, token: str = "") -> dict:
    """通过 Vercel CLI 部署。

    前置条件:
      1. 项目目录包含 package.json
      2. npm install -g vercel (或 npx vercel)
    """
    # 检查 vercel CLI (非阻塞)
    try:
        vercel_check = await asyncio.to_thread(
            subprocess.run,
            ["npx", "vercel", "--version"],
            capture_output=True, text=True, timeout=15,
        )
        if vercel_check.returncode != 0:
            return DeployResponse(
                success=False,
                error="Vercel CLI 不可用。请在终端运行: npm install -g vercel",
            ).model_dump()
    except FileNotFoundError:
        return DeployResponse(
            success=False,
            error="未找到 npx。请安装 Node.js 后运行: npm install -g vercel",
        ).model_dump()
    except subprocess.TimeoutExpired:
        return DeployResponse(
            success=False,
            error="检查 Vercel CLI 超时。请检查网络连接。",
        ).model_dump()

    # 检查 package.json
    if not (project_dir / "package.json").exists():
        return DeployResponse(
            success=False,
            error="项目目录缺少 package.json。Vercel 部署需要 npm 项目。",
        ).model_dump()

    # 构建环境变量
    env = {}
    if token:
        env["VERCEL_TOKEN"] = token

    try:
        # 执行 vercel deploy --prod (非阻塞)
        result = await asyncio.to_thread(
            subprocess.run,
            ["npx", "vercel", "--prod", "--confirm", "--no-clipboard"],
            capture_output=True, text=True, timeout=120,
            cwd=str(project_dir),
            env={**os.environ, **env},
        )

        # 从输出中提取部署 URL（通用 URL 提取，支持多平台）
        stdout = result.stdout + result.stderr
        # Try multiple known URL patterns
        url_patterns = [
            r"https://[^\s]+\.vercel\.app",
            r"https://[^\s]+\.netlify\.app",
            r"https://[^\s]+\.pages\.dev",  # Cloudflare Pages
        ]
        url = None
        for pattern in url_patterns:
            url_match = re.search(pattern, stdout)
            if url_match:
                url = url_match.group(0)
                break

        # Generic fallback: find any URL-like string after "Production:" or "https://"
        if not url:
            for line in stdout.split("\n"):
                for word in line.split():
                    if word.startswith("https://") and "." in word[8:]:
                        url = word.rstrip(".,;")
                        break
                if url:
                    break

        if url:
            return DeployResponse(
                success=True,
                url=url,
                platform="vercel",
            ).model_dump()

        return DeployResponse(
            success=False,
            error=f"部署可能成功但无法提取 URL。\n{stdout[:500]}",
        ).model_dump()

    except subprocess.TimeoutExpired:
        return DeployResponse(
            success=False,
            error="部署超时（120s）。请检查网络连接或 Vercel 状态。",
        ).model_dump()
    except Exception as e:
        return DeployResponse(
            success=False,
            error=str(e),
        ).model_dump()


async def _deploy_netlify(project_dir: Path, token: str = "") -> dict:
    """通过 Netlify CLI 部署。

    前置条件:
      1. 项目目录包含构建产物（dist/ 或 build/）
      2. npm install -g netlify-cli
    """
    # 检查 netlify CLI (非阻塞)
    try:
        netlify_check = await asyncio.to_thread(
            subprocess.run,
            ["npx", "netlify", "--version"],
            capture_output=True, text=True, timeout=15,
        )
        if netlify_check.returncode != 0:
            return DeployResponse(
                success=False,
                error="Netlify CLI 不可用。请在终端运行: npm install -g netlify-cli",
            ).model_dump()
    except FileNotFoundError:
        return DeployResponse(
            success=False,
            error="未找到 npx。请安装 Node.js 后运行: npm install -g netlify-cli",
        ).model_dump()
    except subprocess.TimeoutExpired:
        return DeployResponse(
            success=False,
            error="检查 Netlify CLI 超时。请检查网络连接。",
        ).model_dump()

    # 确定发布目录
    publish_dir = project_dir
    for candidate in ["dist", "build", "out"]:
        if (project_dir / candidate).is_dir():
            publish_dir = project_dir / candidate
            break

    try:
        # 构建命令
        cmd = [
            "npx", "netlify", "deploy",
            "--dir", str(publish_dir),
            "--prod",
        ]
        if token:
            cmd.extend(["--auth", token])

        result = await asyncio.to_thread(
            subprocess.run,
            cmd,
            capture_output=True, text=True, timeout=120,
            cwd=str(project_dir),
        )

        stdout = result.stdout + result.stderr

        # 提取 URL
        url_match = re.search(
            r"https://[^\s]+\.netlify\.app",
            stdout,
        )

        if url_match:
            return DeployResponse(
                success=True,
                url=url_match.group(0),
                platform="netlify",
            ).model_dump()

        # Fallback: 查找 "Website URL:" 行
        for line in stdout.split("\n"):
            if "Website" in line or "url" in line.lower():
                url = line.split()[-1].strip()
                if url.startswith("http"):
                    return DeployResponse(
                        success=True,
                        url=url,
                        platform="netlify",
                    ).model_dump()

        return DeployResponse(
            success=False,
            error=f"部署可能成功但无法提取 URL。\n{stdout[:500]}",
        ).model_dump()

    except subprocess.TimeoutExpired:
        return DeployResponse(
            success=False,
            error="部署超时（120s）。请检查网络连接或 Netlify 状态。",
        ).model_dump()
    except Exception as e:
        return DeployResponse(
            success=False,
            error=str(e),
        ).model_dump()
