#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
将 docs/research/ 下的三篇 Markdown 研究报告转换为学术格式的 DOCX 文档。
包含 matplotlib 生成的图表嵌入。

三篇文档：
  1. poiesis-market-assessment-2026-07-24.md  → 竞争定位与市场评估
  2. poiesis-forward-assessment-2026-07-24.md  → 预期评估：12-18个月愿景
  3. poiesis-value-assessment-2026-07-24.md    → 多维度价值评估
"""

import sys
import re
import os
import io
from pathlib import Path

# Fix Windows console encoding
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8')
from docx import Document
from docx.shared import Pt, Cm, Inches, RGBColor, Emu
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.enum.section import WD_ORIENT
from docx.enum.table import WD_TABLE_ALIGNMENT
from docx.oxml.ns import qn, nsdecls
from docx.oxml import parse_xml
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.font_manager as fm
import numpy as np

# ============================================================
# GLOBAL CONFIG
# ============================================================
BASE_DIR = Path(__file__).parent
CHARTS_DIR = BASE_DIR / "charts"
CHARTS_DIR.mkdir(exist_ok=True)

# Font configuration
BODY_FONT = 'SimSun'          # 宋体 - body text
HEADING_FONT = 'SimHei'       # 黑体 - headings
MONO_FONT = 'STFangsong'      # 仿宋 - code/quotes
CHART_FONT = 'Microsoft YaHei'  # 图表字体

# Register CJK fonts for matplotlib
for fname in fm.findSystemFonts():
    try:
        fm.fontManager.addfont(fname)
    except:
        pass

plt.rcParams['font.family'] = 'sans-serif'
# Try multiple CJK fonts in order of preference
for font_candidate in ['Microsoft YaHei', 'Noto Sans SC', 'SimHei', 'STXihei']:
    try:
        plt.rcParams['font.sans-serif'] = [font_candidate, 'DejaVu Sans']
        fig = plt.figure()
        fig.text(0.5, 0.5, '测试', fontsize=12)
        plt.close(fig)
        break
    except:
        continue

plt.rcParams['axes.unicode_minus'] = False

# Academic colors (subdued, professional)
COLOR_PALETTE = ['#2c3e50', '#c0392b', '#2980b9', '#27ae60', '#8e44ad', '#d35400', '#16a085']
COLOR_POIESIS = '#c0392b'     # terracotta red for Poiesis
COLOR_WEBFLOW = '#2980b9'     # blue
COLOR_FF = '#27ae60'          # green
COLOR_WEBSTUDIO = '#8e44ad'   # purple
COLOR_NEUTRAL = '#7f8c8d'     # grey

# ============================================================
# CHART GENERATION FUNCTIONS
# ============================================================

def save_chart(fig, name):
    """Save chart to charts directory, return path."""
    path = CHARTS_DIR / name
    fig.savefig(str(path), dpi=200, bbox_inches='tight', facecolor='white', edgecolor='none')
    plt.close(fig)
    return path


def chart_competition_comparison():
    """Radar-like horizontal bar chart: 竞品核心维度对比"""
    categories = [
        '多平台覆盖',
        '架构深度',
        '代码双向同步',
        '设计系统原生集成',
        'AI 能力',
        '协作功能',
        '开源性',
        '社区生态规模',
    ]
    # Scores out of 10
    poiesis_now = [9, 7, 9, 8, 0, 0, 10, 1]
    poiesis_target = [9, 9, 10, 9, 8, 8, 10, 5]
    webflow = [2, 1, 1, 6, 7, 8, 0, 10]
    flutterflow = [4, 4, 2, 5, 6, 8, 0, 7]
    webstudio = [2, 1, 5, 5, 0, 3, 10, 4]

    x = np.arange(len(categories))
    width = 0.15

    fig, ax = plt.subplots(figsize=(12, 6))
    bars1 = ax.bar(x - 2*width, poiesis_now, width, label='Poiesis (当前 α)', color=COLOR_POIESIS, alpha=0.7)
    bars2 = ax.bar(x - width, poiesis_target, width, label='Poiesis (12-18月预期)', color=COLOR_POIESIS, alpha=1.0)
    bars3 = ax.bar(x, flutterflow, width, label='FlutterFlow', color=COLOR_FF, alpha=0.8)
    bars4 = ax.bar(x + width, webflow, width, label='Webflow', color=COLOR_WEBFLOW, alpha=0.8)
    bars5 = ax.bar(x + 2*width, webstudio, width, label='WebStudio', color=COLOR_WEBSTUDIO, alpha=0.8)

    ax.set_ylabel('评分（满分 10）', fontsize=11)
    ax.set_title('图 1：竞品核心维度对比', fontsize=14, fontweight='bold', pad=15)
    ax.set_xticks(x)
    ax.set_xticklabels(categories, fontsize=10)
    ax.legend(loc='upper right', fontsize=8, ncol=2)
    ax.set_ylim(0, 11)
    ax.grid(axis='y', alpha=0.3, linestyle='--')
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)

    return save_chart(fig, 'chart_competition_comparison.png')


def chart_market_positioning():
    """Scatter plot: 市场定位图 - 架构深度 vs 平台覆盖"""
    fig, ax = plt.subplots(figsize=(10, 8))

    # Products: (platform_coverage, architecture_depth, label, color, size)
    products = [
        (3, 1.5, 'Webflow', COLOR_WEBFLOW, 150),
        (4, 3.5, 'FlutterFlow', COLOR_FF, 120),
        (2.5, 1, 'WebStudio', COLOR_WEBSTUDIO, 100),
        (8.5, 7, 'Poiesis\n(当前α)', COLOR_POIESIS, 200),
        (9, 9, 'Poiesis\n(12-18月预期)', COLOR_POIESIS, 250),
    ]

    for x, y, label, color, size in products:
        ax.scatter(x, y, c=color, s=size, alpha=0.8, edgecolors='white', linewidth=2, zorder=5)
        ax.annotate(label, (x, y), textcoords="offset points", xytext=(0, 15),
                    ha='center', fontsize=10, fontweight='bold', color=color)

    ax.set_xlabel('平台覆盖广度', fontsize=12, labelpad=10)
    ax.set_ylabel('应用架构深度', fontsize=12, labelpad=10)
    ax.set_title('图 2：市场定位图 — 架构深度 vs 平台覆盖广度', fontsize=14, fontweight='bold', pad=15)

    # Annotate quadrants
    ax.axhline(y=4, color='gray', linestyle='--', alpha=0.3)
    ax.axvline(x=5, color='gray', linestyle='--', alpha=0.3)
    ax.text(1.5, 8.5, '利基架构工具', fontsize=9, color='gray', alpha=0.7)
    ax.text(7.5, 8.5, '全平台架构平台\n（蓝海）', fontsize=9, color=COLOR_POIESIS, alpha=0.7, fontweight='bold')
    ax.text(1.5, 0.5, '利基建站工具', fontsize=9, color='gray', alpha=0.7)
    ax.text(7.5, 0.5, '全平台建站工具', fontsize=9, color='gray', alpha=0.7)

    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)

    return save_chart(fig, 'chart_market_positioning.png')


def chart_competitive_evolution():
    """竞争格局演化 2026→2028"""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # 2026
    products_2026 = [
        (2.5, 1.5, 'Webflow', COLOR_WEBFLOW, 120),
        (4, 3.5, 'FlutterFlow', COLOR_FF, 100),
        (2, 1, 'WebStudio', COLOR_WEBSTUDIO, 80),
        (8, 6.5, 'Poiesis ★', COLOR_POIESIS, 200),
    ]
    for x, y, label, color, size in products_2026:
        ax1.scatter(x, y, c=color, s=size, alpha=0.8, edgecolors='white', linewidth=2, zorder=5)
        ax1.annotate(label, (x, y), textcoords="offset points", xytext=(0, 12),
                    ha='center', fontsize=10, fontweight='bold', color=color)
    ax1.set_title('2026 当前', fontsize=13, fontweight='bold')
    ax1.set_xlim(0, 10); ax1.set_ylim(0, 10)
    ax1.set_xlabel('平台覆盖', fontsize=10)
    ax1.set_ylabel('架构深度', fontsize=10)
    ax1.spines['top'].set_visible(False); ax1.spines['right'].set_visible(False)

    # 2028
    products_2028 = [
        (3.5, 2, 'Webflow', COLOR_WEBFLOW, 130),
        (5, 4.5, 'FlutterFlow', COLOR_FF, 110),
        (3, 1.5, 'WebStudio', COLOR_WEBSTUDIO, 90),
        (4, 8, 'Atsign AI\nArchitect', '#e67e22', 100),
        (9.5, 9, 'Poiesis ★', COLOR_POIESIS, 250),
    ]
    for x, y, label, color, size in products_2028:
        ax2.scatter(x, y, c=color, s=size, alpha=0.8, edgecolors='white', linewidth=2, zorder=5)
        ax2.annotate(label, (x, y), textcoords="offset points", xytext=(0, 12),
                    ha='center', fontsize=10, fontweight='bold', color=color)
    ax2.set_title('2028 预期', fontsize=13, fontweight='bold')
    ax2.set_xlim(0, 10); ax2.set_ylim(0, 10)
    ax2.set_xlabel('平台覆盖', fontsize=10)
    ax2.spines['top'].set_visible(False); ax2.spines['right'].set_visible(False)

    fig.suptitle('图 3：竞争格局演化（2026→2028）', fontsize=14, fontweight='bold', y=1.01)
    return save_chart(fig, 'chart_competitive_evolution.png')


def chart_poiesis_ai_architecture():
    """Poiesis AI 体系架构图"""
    fig, ax = plt.subplots(figsize=(12, 6))
    ax.set_xlim(0, 12)
    ax.set_ylim(0, 6)
    ax.axis('off')

    # Title
    ax.text(6, 5.7, 'Poiesis AI 体系', fontsize=16, fontweight='bold', ha='center', va='center',
            color=COLOR_POIESIS)

    # Left box - AI 架构助手
    left_box = plt.Rectangle((0.5, 0.5), 5, 4.5, fill=True, facecolor='#fdf2f2', edgecolor=COLOR_POIESIS, linewidth=2, alpha=0.8)
    ax.add_patch(left_box)
    ax.text(3, 4.5, 'AI 架构助手', fontsize=13, fontweight='bold', ha='center', va='center', color=COLOR_POIESIS)
    arch_items = ['• 建议组件拆分', '• 数据流设计', '• Store 结构建模', '• 路由/状态机建议', '• 架构最佳实践', '• 依赖关系分析']
    for i, item in enumerate(arch_items):
        ax.text(3, 3.8 - i*0.55, item, fontsize=10, ha='center', va='center', color='#333')

    # Right box - AI 设计助手
    right_box = plt.Rectangle((6.5, 0.5), 5, 4.5, fill=True, facecolor='#f0f4fc', edgecolor='#2980b9', linewidth=2, alpha=0.8)
    ax.add_patch(right_box)
    ax.text(9, 4.5, 'AI 设计助手', fontsize=13, fontweight='bold', ha='center', va='center', color='#2980b9')
    design_items = ['• Cloud Design 风格设计生成', '• Open Design 风格设计生成', '• Token 体系补全', '• Variant 自动生成', '• 设计一致性校验']
    for i, item in enumerate(design_items):
        ax.text(9, 3.8 - i*0.55, item, fontsize=10, ha='center', va='center', color='#333')

    # Bottom philosophy
    ax.text(6, 0.2, '核心理念：AI 是工具，人是决策者 —— 人思考 → AI 补全执行 → 人审核 → Poiesis 编译输出',
            fontsize=11, ha='center', va='center', fontweight='bold',
            bbox=dict(boxstyle='round,pad=0.5', facecolor='#f9f9f9', edgecolor='#ccc', alpha=0.9))

    ax.set_title('图 4：Poiesis AI 架构体系设计', fontsize=14, fontweight='bold', pad=20)
    return save_chart(fig, 'chart_ai_architecture.png')


def chart_financial_scenarios():
    """财务情景分析：期望值瀑布图"""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Left: Scenario comparison
    scenarios = ['保守情景\n(40%)', '中等情景\n(35%)', '乐观情景\n(15%)', '极乐观\n(5%)']
    exit_values = [0.025, 1.5, 30, 500]  # in millions
    arr_values = [0.0025, 0.175, 5.5, 50]
    colors = ['#95a5a6', '#f39c12', '#27ae60', '#8e44ad']

    x = np.arange(len(scenarios))
    width = 0.3
    bars1 = ax1.bar(x - width/2, exit_values, width, label='退出价值', color=COLOR_POIESIS, alpha=0.8)
    bars2 = ax1.bar(x + width/2, arr_values, width, label='年收入 (ARR)', color=COLOR_FF, alpha=0.8)
    ax1.set_ylabel('金额（百万美元）', fontsize=11)
    ax1.set_title('各情景财务指标', fontsize=13, fontweight='bold')
    ax1.set_xticks(x)
    ax1.set_xticklabels(scenarios, fontsize=9)
    ax1.legend(fontsize=9)
    ax1.set_yscale('log')
    ax1.grid(axis='y', alpha=0.3, linestyle='--')
    ax1.spines['top'].set_visible(False); ax1.spines['right'].set_visible(False)

    # Add value labels
    for bar, val in zip(bars1, exit_values):
        label = f'${val}M' if val >= 1 else f'${val*1000:.0f}K'
        ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height()*1.1, label,
                ha='center', va='bottom', fontsize=8)
    for bar, val in zip(bars2, arr_values):
        label = f'${val}M' if val >= 1 else f'${val*1000:.0f}K'
        ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height()*1.1, label,
                ha='center', va='bottom', fontsize=8)

    # Right: Expected value
    ev_labels = ['保守', '中等', '乐观', '期望值\n(保守估计)']
    ev_values = [0.01, 0.525, 4.5, 5.035]
    ev_colors = ['#95a5a6', '#f39c12', '#27ae60', COLOR_POIESIS]
    bars = ax2.bar(ev_labels, ev_values, color=ev_colors, alpha=0.8, width=0.5)
    ax2.set_ylabel('期望值（百万美元）', fontsize=11)
    ax2.set_title('财务期望值拆分', fontsize=13, fontweight='bold')
    ax2.grid(axis='y', alpha=0.3, linestyle='--')
    ax2.spines['top'].set_visible(False); ax2.spines['right'].set_visible(False)
    for bar in bars:
        ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.1,
                f'${bar.get_height():.2f}M', ha='center', va='bottom', fontsize=10, fontweight='bold')

    fig.suptitle('图 5：财务情景分析', fontsize=14, fontweight='bold', y=1.02)
    return save_chart(fig, 'chart_financial_scenarios.png')


def chart_user_scale_value():
    """用户规模-价值曲线"""
    fig, ax = plt.subplots(figsize=(11, 7))

    users = np.array([0, 50, 100, 300, 500, 1000, 3000, 5000, 10000, 50000, 100000, 500000, 1000000])
    # Personal value (saturates quickly) - normalized 0-100
    personal = 85 * (1 - np.exp(-users / 80))
    # Financial value (grows slowly then exponentially) - normalized
    financial = 100 / (1 + np.exp(-(np.log10(users + 1) - 3.5) * 3))
    # Ecosystem value (linear growth)
    ecosystem = np.clip(users / 1000 * 10, 0, 100)

    ax.plot(users, personal, '--', color='#e74c3c', linewidth=2.5, label='个人价值（技能/品牌/知识）')
    ax.plot(users, financial, '-', color='#27ae60', linewidth=2.5, label='财务价值')
    ax.plot(users, ecosystem, ':', color='#2980b9', linewidth=2.5, label='生态价值')

    # Annotations
    ax.annotate('个人价值在极小规模\n已接近饱和', xy=(100, 80), xytext=(500, 55),
                arrowprops=dict(arrowstyle='->', color='#e74c3c', lw=1.5),
                fontsize=10, color='#e74c3c', fontweight='bold')
    ax.annotate('财务价值随规模\n指数增长', xy=(50000, 80), xytext=(15000, 45),
                arrowprops=dict(arrowstyle='->', color='#27ae60', lw=1.5),
                fontsize=10, color='#27ae60', fontweight='bold')

    ax.set_xlabel('用户规模', fontsize=12)
    ax.set_ylabel('价值指数（归一化）', fontsize=12)
    ax.set_title('图 6：用户规模-价值曲线', fontsize=14, fontweight='bold', pad=15)
    ax.set_xscale('log')
    ax.set_xlim(5, 2e6)
    ax.set_ylim(-5, 105)
    ax.legend(fontsize=11, loc='lower right')
    ax.grid(alpha=0.3, linestyle='--')
    ax.spines['top'].set_visible(False); ax.spines['right'].set_visible(False)

    # Vertical reference lines
    for x_val, label in [(100, '100'), (1000, '1K'), (10000, '10K'), (100000, '100K')]:
        ax.axvline(x=x_val, color='gray', alpha=0.15, linestyle='--')

    return save_chart(fig, 'chart_user_scale_value.png')


def chart_roi_comparison():
    """ROI 对比：Poiesis vs 替代选择"""
    fig, ax = plt.subplots(figsize=(10, 6))

    options = ['接外包项目', '普通开发工作\n(1年)', '普通 SaaS\n(期望值)', 'Poiesis\n最坏情景', 'Poiesis\n中等情景']
    roi_values = [1, 3, 0.5, 100, 300]  # ROI multiples
    colors_bar = ['#95a5a6', '#bdc3c7', '#7f8c8d', COLOR_POIESIS, '#e74c3c']

    bars = ax.barh(options, roi_values, color=colors_bar, alpha=0.85, height=0.6)
    ax.axvline(x=1, color='red', linestyle='--', alpha=0.5, linewidth=1, label='盈亏平衡线 (ROI=1x)')
    ax.set_xlabel('ROI（投入产出倍数）', fontsize=12)
    ax.set_title('图 7：ROI 对比 — Poiesis vs 替代选择', fontsize=14, fontweight='bold', pad=15)

    for bar, val in zip(bars, roi_values):
        label = f'{val}x' if val >= 1 else f'{val}x'
        ax.text(bar.get_width() + 3, bar.get_y() + bar.get_height()/2,
                label, va='center', fontsize=12, fontweight='bold', color='#2c3e50')

    ax.legend(fontsize=10, loc='lower right')
    ax.grid(axis='x', alpha=0.3, linestyle='--')
    ax.spines['top'].set_visible(False); ax.spines['right'].set_visible(False)
    ax.set_xlim(0, 400)

    return save_chart(fig, 'chart_roi_comparison.png')


def chart_opportunity_cost():
    """机会成本对比"""
    fig, ax = plt.subplots(figsize=(10, 6))

    categories = ['AI 费用\n(显性成本)', '能力积累\n(架构/AI编排)', '职业溢价\n(3-5年)', '品牌资产', '产品收入\n潜力']
    poiesis_vals = [-100, 95, 90, 80, 70]
    alternatives_vals = [0, 30, 40, 20, 30]

    x = np.arange(len(categories))
    width = 0.3

    bars1 = ax.bar(x - width/2, poiesis_vals, width, label='Poiesis', color=COLOR_POIESIS, alpha=0.85)
    bars2 = ax.bar(x + width/2, alternatives_vals, width, label='替代选择（外包/普通工作）', color=COLOR_NEUTRAL, alpha=0.7)

    ax.set_ylabel('价值评分（满分 100）', fontsize=11)
    ax.set_title('图 8：机会成本对比 — Poiesis vs 替代选择', fontsize=14, fontweight='bold', pad=15)
    ax.set_xticks(x)
    ax.set_xticklabels(categories, fontsize=10)
    ax.legend(fontsize=10)
    ax.grid(axis='y', alpha=0.3, linestyle='--')
    ax.spines['top'].set_visible(False); ax.spines['right'].set_visible(False)
    ax.set_ylim(-20, 110)

    return save_chart(fig, 'chart_opportunity_cost.png')


def chart_value_certainty():
    """价值-确定性矩阵"""
    fig, ax = plt.subplots(figsize=(10, 8))

    items = [
        (95, 95, '架构决策\n系统思维', '#e74c3c', 180),
        (90, 90, 'AI编排\n指挥能力', '#e74c3c', 170),
        (85, 75, '产品品味\n设计判断', '#e67e22', 140),
        (80, 60, '问题分解\n任务拆解', '#f39c12', 130),
        (70, 55, '个人品牌\n(AI Orchestrator)', '#27ae60', 130),
        (55, 50, '社区治理\n沟通推广', '#2980b9', 120),
        (40, 30, '生态影响', '#8e44ad', 100),
        (30, 15, '品类定义', '#8e44ad', 90),
        (50, 20, '财务回报', COLOR_POIESIS, 140),
    ]

    for x, y, label, color, size in items:
        ax.scatter(x, y, c=color, s=size, alpha=0.8, edgecolors='white', linewidth=1.5, zorder=5)
        ax.annotate(label, (x, y), textcoords="offset points", xytext=(0, -20),
                    ha='center', fontsize=9, fontweight='bold', color=color)

    ax.set_xlabel('量级（价值大小）', fontsize=12, labelpad=10)
    ax.set_ylabel('确定性', fontsize=12, labelpad=10)
    ax.set_title('图 9：价值-确定性矩阵', fontsize=14, fontweight='bold', pad=15)

    # Quadrant lines
    ax.axhline(y=50, color='gray', linestyle='--', alpha=0.3)
    ax.axvline(x=50, color='gray', linestyle='--', alpha=0.3)
    ax.text(25, 88, '高确定性\n中量级', fontsize=9, color='gray', ha='center')
    ax.text(80, 88, '高确定性\n大量级', fontsize=9, color=COLOR_POIESIS, ha='center', fontweight='bold')
    ax.text(25, 25, '低确定性\n中量级', fontsize=9, color='gray', ha='center')
    ax.text(80, 25, '低确定性\n大量级', fontsize=9, color='gray', ha='center')

    ax.set_xlim(0, 105); ax.set_ylim(0, 105)
    ax.spines['top'].set_visible(False); ax.spines['right'].set_visible(False)

    return save_chart(fig, 'chart_value_certainty.png')


def chart_strategic_phases():
    """战略路线图阶段图"""
    fig, ax = plt.subplots(figsize=(14, 5))
    ax.set_xlim(0, 18)
    ax.set_ylim(0, 6)
    ax.axis('off')

    phases = [
        {
            'title': 'Phase 1：基础闭环\n(0-6个月)',
            'x': 0.5, 'w': 5, 'color': '#e74c3c',
            'items': ['完善语言特性\n(@for, lifecycle, 事件中继)',
                     'React target 生产级',
                     '一个标杆 Demo',
                     'Token 体系 + 设计系统完整']
        },
        {
            'title': 'Phase 2：AI MVP\n(6-12个月)',
            'x': 6, 'w': 5.5, 'color': '#f39c12',
            'items': ['AI 架构助手 MVP',
                     'AI 设计助手 MVP',
                     'Flutter target beta',
                     'Git 协作基础支持']
        },
        {
            'title': 'Phase 3：协作+生态\n(12-18个月)',
            'x': 12, 'w': 5.5, 'color': '#27ae60',
            'items': ['实时协作',
                     '语义化 Git diff',
                     'Flutter target 生产级',
                     '社区建设 + 开放生态']
        },
    ]

    for p in phases:
        rect = plt.Rectangle((p['x'], 1.5), p['w'], 3, fill=True,
                            facecolor=p['color'], edgecolor='white', linewidth=2, alpha=0.15)
        ax.add_patch(rect)
        rect2 = plt.Rectangle((p['x'], 3.5), p['w'], 0.8, fill=True,
                             facecolor=p['color'], edgecolor='white', linewidth=2, alpha=0.9)
        ax.add_patch(rect2)
        ax.text(p['x'] + p['w']/2, 3.9, p['title'], ha='center', va='center',
                fontsize=11, fontweight='bold', color='white')

        for i, item in enumerate(p['items']):
            ax.text(p['x'] + 0.3, 3.2 - i*0.65, f'• {item}', fontsize=9, va='center', color='#333')

    # Arrow between phases
    ax.annotate('', xy=(5.9, 3.9), xytext=(5.6, 3.9),
                arrowprops=dict(arrowstyle='->', color='#333', lw=2))
    ax.annotate('', xy=(11.9, 3.9), xytext=(11.6, 3.9),
                arrowprops=dict(arrowstyle='->', color='#333', lw=2))

    ax.set_title('图 10：建议战略节奏（12-18 个月路线图）', fontsize=14, fontweight='bold', pad=20)
    return save_chart(fig, 'chart_strategic_phases.png')


def chart_risk_heatmap():
    """风险热力图"""
    fig, ax = plt.subplots(figsize=(10, 6))

    risks = [
        ('AI 执行质量', 5, 5),
        ('编译器性能', 4, 3),
        ('Cloud/Open Design\n整合复杂度', 4, 3),
        ('"架构师"\n受众规模', 3, 4),
        ('品类教育成本', 4, 5),
        ('AI 能力缺失', 5, 4),
        ('编译质量\n是生死线', 5, 5),
        ('范围蔓延', 3, 3),
        ('Google/FF\n资源碾压', 4, 3),
        ('开源商业模式\n未验证', 3, 4),
    ]

    for risk, severity, likelihood in risks:
        color_intensity = (severity * likelihood) / 25
        color = plt.cm.RdYlGn_r(0.2 + color_intensity * 0.8)
        ax.scatter(severity, likelihood, s=severity*likelihood*50, c=[color],
                  alpha=0.8, edgecolors='white', linewidth=1.5, zorder=5)
        ax.annotate(risk, (severity, likelihood), textcoords="offset points",
                   xytext=(0, -18), ha='center', fontsize=9)

    ax.set_xlabel('严重程度', fontsize=12, labelpad=10)
    ax.set_ylabel('发生概率', fontsize=12, labelpad=10)
    ax.set_title('图 11：风险评估矩阵（气泡大小 = 综合风险等级）', fontsize=14, fontweight='bold', pad=15)
    ax.set_xlim(0.5, 6); ax.set_ylim(0.5, 6)
    ax.set_xticks([1, 2, 3, 4, 5])
    ax.set_yticks([1, 2, 3, 4, 5])
    ax.spines['top'].set_visible(False); ax.spines['right'].set_visible(False)
    ax.grid(alpha=0.3, linestyle='--')

    # Add quadrant labels
    ax.text(5.3, 5.3, '紧急处理区', fontsize=9, color='#c0392b', fontweight='bold')
    ax.text(1.5, 1.5, '监控区', fontsize=9, color='#27ae60')
    ax.text(5.3, 1.5, '缓解区', fontsize=9, color='#f39c12')
    ax.text(1.5, 5.3, '预防区', fontsize=9, color='#f39c12')

    return save_chart(fig, 'chart_risk_heatmap.png')


def generate_all_charts():
    """Generate all charts, return dict of chart_name -> path."""
    charts = {}
    chart_funcs = [
        ('competition_comparison', chart_competition_comparison),
        ('market_positioning', chart_market_positioning),
        ('competitive_evolution', chart_competitive_evolution),
        ('ai_architecture', chart_poiesis_ai_architecture),
        ('financial_scenarios', chart_financial_scenarios),
        ('user_scale_value', chart_user_scale_value),
        ('roi_comparison', chart_roi_comparison),
        ('opportunity_cost', chart_opportunity_cost),
        ('value_certainty', chart_value_certainty),
        ('strategic_phases', chart_strategic_phases),
        ('risk_heatmap', chart_risk_heatmap),
    ]
    for name, func in chart_funcs:
        try:
            path = func()
            charts[name] = path
            print(f"  ✓ Generated chart: {path.name}")
        except Exception as e:
            print(f"  ✗ Failed to generate chart '{name}': {e}")
    return charts


# ============================================================
# DOCX STYLING HELPERS
# ============================================================

def set_cell_font(cell, text, font_name=BODY_FONT, size=10, bold=False, color=None, alignment=None):
    """Set font properties for a table cell."""
    cell.text = ''
    p = cell.paragraphs[0]
    run = p.add_run(text)
    run.font.name = font_name
    run._element.rPr.rFonts.set(qn('w:eastAsia'), font_name)
    run.font.size = Pt(size)
    run.font.bold = bold
    if color:
        run.font.color.rgb = RGBColor(*color)
    if alignment is not None:
        p.alignment = alignment


def add_formatted_paragraph(doc, text, font_name=BODY_FONT, size=12, bold=False,
                            alignment=None, space_after=6, first_line_indent=None,
                            color=None, italic=False):
    """Add a paragraph with proper CJK font settings."""
    p = doc.add_paragraph()
    run = p.add_run(text)
    run.font.name = font_name
    run._element.rPr.rFonts.set(qn('w:eastAsia'), font_name)
    run.font.size = Pt(size)
    run.font.bold = bold
    run.font.italic = italic
    if color:
        run.font.color.rgb = RGBColor(*color)
    if alignment is not None:
        p.alignment = alignment
    pf = p.paragraph_format
    pf.space_after = Pt(space_after)
    pf.line_spacing = 1.5
    if first_line_indent:
        pf.first_line_indent = Cm(first_line_indent)
    return p


def add_heading_styled(doc, text, level=1):
    """Add a heading with proper CJK font."""
    h = doc.add_heading(text, level=level)
    # Set CJK font for heading
    for run in h.runs:
        run.font.name = HEADING_FONT
        run._element.rPr.rFonts.set(qn('w:eastAsia'), HEADING_FONT)
        if level == 1:
            run.font.size = Pt(18)
        elif level == 2:
            run.font.size = Pt(15)
        elif level == 3:
            run.font.size = Pt(13)
    return h


def add_table_from_md(doc, md_table_text):
    """Parse a markdown table and add it as a formatted docx table."""
    lines = [l.strip() for l in md_table_text.strip().split('\n') if l.strip() and not l.strip().startswith('|---')]
    if len(lines) < 2:
        return

    # Parse rows
    rows = []
    for line in lines:
        cells = [c.strip() for c in line.strip('|').split('|')]
        rows.append(cells)

    if not rows:
        return

    ncols = len(rows[0])
    table = doc.add_table(rows=len(rows), cols=ncols, style='Table Grid')
    table.alignment = WD_TABLE_ALIGNMENT.CENTER

    for i, row_data in enumerate(rows):
        for j, cell_text in enumerate(row_data):
            cell = table.rows[i].cells[j]
            if i == 0:
                # Header row
                set_cell_font(cell, cell_text, font_name=HEADING_FONT, size=9, bold=True,
                            color=(255, 255, 255))
                # Dark background for header
                shading = parse_xml(f'<w:shd {nsdecls("w")} w:fill="2c3e50"/>')
                cell._element.get_or_add_tcPr().append(shading)
            else:
                set_cell_font(cell, cell_text, font_name=BODY_FONT, size=9)
                # Alternating row background
                if i % 2 == 0:
                    shading = parse_xml(f'<w:shd {nsdecls("w")} w:fill="f5f6fa"/>')
                    cell._element.get_or_add_tcPr().append(shading)

    doc.add_paragraph()  # spacer
    return table


def format_code_block(doc, text):
    """Add a formatted code block."""
    for line in text.strip().split('\n'):
        p = doc.add_paragraph()
        run = p.add_run(line)
        run.font.name = MONO_FONT
        run._element.rPr.rFonts.set(qn('w:eastAsia'), MONO_FONT)
        run.font.size = Pt(9)
        p.paragraph_format.space_after = Pt(0)
        p.paragraph_format.space_before = Pt(0)
        p.paragraph_format.line_spacing = 1.2
        # Light grey background via shading
        shading = parse_xml(f'<w:shd {nsdecls("w")} w:fill="f4f4f4"/>')
        p._element.get_or_add_pPr().append(shading)


def add_blockquote(doc, text):
    """Add a blockquote-style paragraph."""
    p = doc.add_paragraph()
    run = p.add_run(text)
    run.font.name = BODY_FONT
    run._element.rPr.rFonts.set(qn('w:eastAsia'), BODY_FONT)
    run.font.size = Pt(11)
    run.font.italic = True
    run.font.color.rgb = RGBColor(80, 80, 80)
    p.paragraph_format.left_indent = Cm(1.5)
    p.paragraph_format.right_indent = Cm(1)
    pf = p.paragraph_format
    pf.space_after = Pt(8)
    pf.line_spacing = 1.5


def add_chart_image(doc, chart_path, caption_text, width_inches=5.5):
    """Add a chart image with caption."""
    if not chart_path.exists():
        add_formatted_paragraph(doc, f'[图表缺失：{chart_path.name}]', color=(200, 0, 0), size=10)
        return

    # Center the image
    p_img = doc.add_paragraph()
    p_img.alignment = WD_ALIGN_PARAGRAPH.CENTER
    run_img = p_img.add_run()
    run_img.add_picture(str(chart_path), width=Inches(width_inches))

    # Caption
    p_cap = doc.add_paragraph()
    p_cap.alignment = WD_ALIGN_PARAGRAPH.CENTER
    run_cap = p_cap.add_run(caption_text)
    run_cap.font.name = HEADING_FONT
    run_cap._element.rPr.rFonts.set(qn('w:eastAsia'), HEADING_FONT)
    run_cap.font.size = Pt(9)
    run_cap.font.color.rgb = RGBColor(80, 80, 80)

    doc.add_paragraph()  # spacer


def setup_document():
    """Create a new document with academic formatting."""
    doc = Document()

    # Page setup
    for section in doc.sections:
        section.top_margin = Cm(2.54)
        section.bottom_margin = Cm(2.54)
        section.left_margin = Cm(3.18)
        section.right_margin = Cm(3.18)

    # Set default font
    style = doc.styles['Normal']
    font = style.font
    font.name = BODY_FONT
    font.size = Pt(12)
    style.element.rPr.rFonts.set(qn('w:eastAsia'), BODY_FONT)
    pf = style.paragraph_format
    pf.line_spacing = 1.5

    return doc


def add_title_page(doc, title, subtitle, date, abstract_lines=None):
    """Add an academic title page."""
    # Blank space for title positioning
    for _ in range(4):
        doc.add_paragraph()

    # Title
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    run = p.add_run(title)
    run.font.name = HEADING_FONT
    run._element.rPr.rFonts.set(qn('w:eastAsia'), HEADING_FONT)
    run.font.size = Pt(22)
    run.font.bold = True
    run.font.color.rgb = RGBColor(44, 62, 80)

    # Subtitle
    if subtitle:
        doc.add_paragraph()
        p = doc.add_paragraph()
        p.alignment = WD_ALIGN_PARAGRAPH.CENTER
        run = p.add_run(subtitle)
        run.font.name = HEADING_FONT
        run._element.rPr.rFonts.set(qn('w:eastAsia'), HEADING_FONT)
        run.font.size = Pt(14)
        run.font.color.rgb = RGBColor(100, 100, 100)

    # Decorative line
    doc.add_paragraph()
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    run = p.add_run('─' * 40)
    run.font.color.rgb = RGBColor(192, 57, 43)

    # Date
    doc.add_paragraph()
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    run = p.add_run(date)
    run.font.name = BODY_FONT
    run._element.rPr.rFonts.set(qn('w:eastAsia'), BODY_FONT)
    run.font.size = Pt(12)
    run.font.color.rgb = RGBColor(100, 100, 100)

    # Abstract
    if abstract_lines:
        doc.add_paragraph()
        doc.add_paragraph()
        p = doc.add_paragraph()
        p.alignment = WD_ALIGN_PARAGRAPH.LEFT
        run = p.add_run('摘  要')
        run.font.name = HEADING_FONT
        run._element.rPr.rFonts.set(qn('w:eastAsia'), HEADING_FONT)
        run.font.size = Pt(14)
        run.font.bold = True

        for line in abstract_lines:
            add_formatted_paragraph(doc, line, size=11, first_line_indent=0.74)

    # Page break
    doc.add_page_break()


def add_header_footer(doc):
    """Add headers and footers to all sections."""
    for section in doc.sections:
        # Header
        header = section.header
        header.is_linked_to_previous = False
        hp = header.paragraphs[0] if header.paragraphs else header.add_paragraph()
        hp.alignment = WD_ALIGN_PARAGRAPH.CENTER
        run = hp.add_run('Poiesis 项目研究报告')
        run.font.name = BODY_FONT
        run._element.rPr.rFonts.set(qn('w:eastAsia'), BODY_FONT)
        run.font.size = Pt(9)
        run.font.color.rgb = RGBColor(150, 150, 150)
        # Add a thin line below header
        p_border = parse_xml(
            f'<w:pBdr {nsdecls("w")}>'
            f'<w:bottom w:val="single" w:sz="4" w:space="1" w:color="CCCCCC"/>'
            f'</w:pBdr>'
        )
        hp._element.get_or_add_pPr().append(p_border)

        # Footer with page number
        footer = section.footer
        footer.is_linked_to_previous = False
        fp = footer.paragraphs[0] if footer.paragraphs else footer.add_paragraph()
        fp.alignment = WD_ALIGN_PARAGRAPH.CENTER
        # Add page number field
        run = fp.add_run()
        fldChar1 = parse_xml(f'<w:fldChar {nsdecls("w")} w:fldCharType="begin"/>')
        run._element.append(fldChar1)
        run2 = fp.add_run()
        instrText = parse_xml(f'<w:instrText {nsdecls("w")} xml:space="preserve"> PAGE </w:instrText>')
        run2._element.append(instrText)
        run3 = fp.add_run()
        fldChar2 = parse_xml(f'<w:fldChar {nsdecls("w")} w:fldCharType="end"/>')
        run3._element.append(fldChar2)
        for r in [run, run2, run3]:
            r.font.name = BODY_FONT
            r._element.rPr.rFonts.set(qn('w:eastAsia'), BODY_FONT)
            r.font.size = Pt(9)


# ============================================================
# MARKDOWN PARSER
# ============================================================

def parse_markdown_structure(md_text):
    """
    Parse markdown into structured blocks.
    Returns list of dicts: {type, content, level (for headings), meta}
    """
    lines = md_text.split('\n')
    blocks = []
    i = 0

    while i < len(lines):
        line = lines[i]

        # Skip empty
        if not line.strip():
            i += 1
            continue

        # Headings
        heading_match = re.match(r'^(#{1,6})\s+(.+)$', line)
        if heading_match:
            level = len(heading_match.group(1))
            content = heading_match.group(2).strip()
            blocks.append({'type': 'heading', 'level': level, 'content': content})
            i += 1
            continue

        # Code blocks (fenced)
        if line.strip().startswith('```'):
            code_lines = []
            i += 1
            while i < len(lines) and not lines[i].strip().startswith('```'):
                code_lines.append(lines[i])
                i += 1
            i += 1  # skip closing ```
            blocks.append({'type': 'code', 'content': '\n'.join(code_lines)})
            continue

        # Tables (detect pipe-delimited lines)
        if '|' in line and i+1 < len(lines) and '---' in lines[i+1]:
            table_lines = []
            while i < len(lines) and '|' in lines[i] and lines[i].strip():
                table_lines.append(lines[i])
                i += 1
            blocks.append({'type': 'table', 'content': '\n'.join(table_lines)})
            continue

        # Blockquotes
        if line.strip().startswith('>'):
            quote_lines = []
            while i < len(lines) and lines[i].strip().startswith('>'):
                quote_text = lines[i].strip().lstrip('>').strip()
                quote_lines.append(quote_text)
                i += 1
            blocks.append({'type': 'blockquote', 'content': '\n'.join(quote_lines)})
            continue

        # Horizontal rules
        if line.strip() in ('---', '***', '___', '- - -'):
            blocks.append({'type': 'hr', 'content': ''})
            i += 1
            continue

        # Regular paragraph
        para_lines = []
        while i < len(lines) and lines[i].strip() and \
              not lines[i].strip().startswith('#') and \
              not lines[i].strip().startswith('```') and \
              not lines[i].strip().startswith('>') and \
              not ('|' in lines[i] and i+1 < len(lines) and '---' in lines[i+1]):
            para_lines.append(lines[i])
            i += 1
        blocks.append({'type': 'paragraph', 'content': '\n'.join(para_lines)})

    return blocks


def process_inline_formatting(text):
    """Process inline markdown formatting: bold, italic, inline code."""
    # Bold: **text**
    text = re.sub(r'\*\*(.+?)\*\*', r'【BOLD】\1【/BOLD】', text)
    # Italic: *text* (but not **)
    text = re.sub(r'(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)', r'【ITALIC】\1【/ITALIC】', text)
    # Inline code: `text`
    text = re.sub(r'`([^`]+?)`', r'【CODE】\1【/CODE】', text)
    return text


def add_inline_formatted_run(paragraph, text_segment, is_bold=False, is_italic=False, is_code=False):
    """Add a run with proper formatting to a paragraph."""
    if is_code:
        font = MONO_FONT
        size = 10
    else:
        font = BODY_FONT
        size = 12

    run = paragraph.add_run(text_segment)
    run.font.name = font
    run._element.rPr.rFonts.set(qn('w:eastAsia'), font)
    run.font.size = Pt(size)
    run.font.bold = is_bold
    run.font.italic = is_italic
    return run


def add_rich_paragraph(doc, text, font_name=BODY_FONT, size=12):
    """Add a paragraph with inline formatting (bold, italic, code)."""
    text = process_inline_formatting(text)
    p = doc.add_paragraph()
    p.paragraph_format.line_spacing = 1.5
    p.paragraph_format.space_after = Pt(6)

    # Split by formatting markers
    segments = re.split(r'(【BOLD】|【/BOLD】|【ITALIC】|【/ITALIC】|【CODE】|【/CODE】)', text)

    is_bold = False
    is_italic = False
    is_code = False

    for seg in segments:
        if seg == '【BOLD】':
            is_bold = True
        elif seg == '【/BOLD】':
            is_bold = False
        elif seg == '【ITALIC】':
            is_italic = True
        elif seg == '【/ITALIC】':
            is_italic = False
        elif seg == '【CODE】':
            is_code = True
        elif seg == '【/CODE】':
            is_code = False
        elif seg.strip():
            add_inline_formatted_run(p, seg, is_bold, is_italic, is_code)

    return p


# ============================================================
# DOCUMENT BUILDERS
# ============================================================

# Map which charts go into which document sections
DOC1_CHARTS = ['competition_comparison', 'market_positioning']
DOC2_CHARTS = ['competitive_evolution', 'ai_architecture', 'strategic_phases', 'risk_heatmap']
DOC3_CHARTS = ['financial_scenarios', 'user_scale_value', 'roi_comparison', 'opportunity_cost', 'value_certainty', 'risk_heatmap']


def build_document(md_path, output_path, title, subtitle, date, abstract_lines,
                   chart_keys, chart_dir):
    """Build a single DOCX from markdown."""
    doc = setup_document()
    add_title_page(doc, title, subtitle, date, abstract_lines)
    add_header_footer(doc)

    # Read markdown
    with open(md_path, 'r', encoding='utf-8') as f:
        md_text = f.read()

    blocks = parse_markdown_structure(md_text)

    chart_index = 0
    chart_inserted_after_headings = {}

    # Map heading text to chart keys for insertion
    heading_to_chart = {
        '竞品深度对比': 'competition_comparison',
        '市场定位图': 'market_positioning',
        '预期竞争格局演化': 'competitive_evolution',
        '预期竞争格局演化（2027-2028）': 'competitive_evolution',
        'AI 架构助手 + AI 设计助手': 'ai_architecture',
        'Poiesis AI 体系': 'ai_architecture',
        '建议的战略节奏': 'strategic_phases',
        '财务价值': 'financial_scenarios',
        '三种财务情景': 'financial_scenarios',
        '规模-价值曲线': 'user_scale_value',
        '用户规模维度分析': 'user_scale_value',
        'ROI 拆解': 'roi_comparison',
        '机会成本分析': 'opportunity_cost',
        '综合价值矩阵': 'value_certainty',
        '风险评估': 'risk_heatmap',
        '战略优势与风险再评估': 'risk_heatmap',
    }

    # Build chart lookup from generated charts
    chart_lookup = {}
    for key in chart_keys:
        path = chart_dir / f'chart_{key}.png'
        if path.exists():
            chart_lookup[key] = path

    chart_captions = {
        'competition_comparison': '竞品核心维度对比（评分制，满分 10 分）',
        'market_positioning': '市场定位图：架构深度 vs 平台覆盖广度（气泡大小代表产品成熟度）',
        'competitive_evolution': '竞争格局演化：2026 年当前状态 vs 2028 年预期状态',
        'ai_architecture': 'Poiesis AI 架构体系设计：AI 架构助手 + AI 设计助手双轨制',
        'financial_scenarios': '财务情景分析：四种情景的收入与退出价值对比',
        'user_scale_value': '用户规模-价值曲线：个人价值在低规模时即饱和，财务价值随规模指数增长',
        'roi_comparison': 'ROI 对比分析：Poiesis 与替代选择的投入产出比较',
        'opportunity_cost': '机会成本对比：Poiesis 在能力积累、职业溢价等维度显著优于替代选择',
        'value_certainty': '价值-确定性矩阵：各类价值的量级与确定性分布',
        'strategic_phases': '建议战略节奏：12-18 个月三阶段路线图',
        'risk_heatmap': '风险评估矩阵：气泡大小代表综合风险等级',
    }

    for block in blocks:
        if block['type'] == 'heading':
            add_heading_styled(doc, block['content'], level=min(block['level'], 3))

            # Check if a chart should be inserted after this heading
            for keyword, chart_key in heading_to_chart.items():
                if keyword in block['content'] and chart_key in chart_lookup and chart_key not in chart_inserted_after_headings:
                    add_chart_image(doc, chart_lookup[chart_key], chart_captions.get(chart_key, ''))
                    chart_inserted_after_headings[chart_key] = True
                    break

        elif block['type'] == 'paragraph':
            text = block['content'].strip()
            if text:
                # Check for list items
                lines = text.split('\n')
                for line in lines:
                    stripped = line.strip()
                    if stripped.startswith('- '):
                        # Unordered list item
                        add_rich_paragraph(doc, '• ' + stripped[2:], size=11)
                    elif re.match(r'^\d+\.', stripped):
                        # Ordered list item
                        add_rich_paragraph(doc, stripped, size=11)
                    else:
                        add_rich_paragraph(doc, stripped)

        elif block['type'] == 'code':
            format_code_block(doc, block['content'])

        elif block['type'] == 'table':
            add_table_from_md(doc, block['content'])

        elif block['type'] == 'blockquote':
            add_blockquote(doc, block['content'])

        elif block['type'] == 'hr':
            p = doc.add_paragraph()
            p.alignment = WD_ALIGN_PARAGRAPH.CENTER
            run = p.add_run('· · ·')
            run.font.color.rgb = RGBColor(180, 180, 180)
            run.font.size = Pt(14)

    # Save
    doc.save(str(output_path))
    print(f"  ✓ Saved: {output_path.name}")


# ============================================================
# MAIN
# ============================================================

def main():
    print("=" * 60)
    print("Poiesis 研究报告 → 学术格式 DOCX 转换器")
    print("=" * 60)

    # Step 1: Generate all charts
    print("\n[1/2] 生成图表...")
    charts = generate_all_charts()

    # Step 2: Build DOCX documents
    print("\n[2/2] 生成 DOCX 文档...")

    doc_configs = [
        {
            'md_path': BASE_DIR / 'poiesis-market-assessment-2026-07-24.md',
            'output_path': BASE_DIR / 'Poiesis_竞争定位与市场评估_2026.docx',
            'title': 'Poiesis 竞争定位与市场评估',
            'subtitle': '当前状态分析报告（2026年7月）',
            'date': '2026年7月24日',
            'abstract_lines': [
                '本报告对 Poiesis 项目在当前时间节点（2026年7月）的竞争定位与市场环境进行系统评估。',
                '报告基于对 Webflow、FlutterFlow、WebStudio 三大对标竞品的深度调研，从多目标编译、架构能力、代码双向同步、设计系统集成等维度进行了横向对比分析。',
                '研究发现：Poiesis 所处的低代码/可视化开发市场规模达 445亿美元（2026年 Gartner 估计），并以 10-19% 的复合年增长率持续扩张。现有竞品在"多目标编译""架构级应用构建能力""可视化-代码双向同步"三个维度上均存在系统性盲区，这构成了 Poiesis 的战略窗口。',
                '关键词：Poiesis；竞争分析；可视化开发；低代码平台；多目标编译；OmniLang',
            ],
            'chart_keys': DOC1_CHARTS,
        },
        {
            'md_path': BASE_DIR / 'poiesis-forward-assessment-2026-07-24.md',
            'output_path': BASE_DIR / 'Poiesis_预期评估与愿景_2026.docx',
            'title': 'Poiesis 预期评估：12-18 个月愿景',
            'subtitle': '基于创始人愿景的前瞻性分析（2026-2028）',
            'date': '2026年7月24日',
            'abstract_lines': [
                '本报告基于项目创始人的愿景陈述，对 Poiesis 项目在未来 12-18 个月的发展方向、功能路线图、竞争格局演化及战略节奏进行前瞻性评估。',
                '报告提出了"AI 时代的架构师工作站"这一核心定位，阐述了 AI 架构助手与 AI 设计助手双轨制 AI 体系的设计思路，并论证了"人做决策、AI 执行实现"这一差异化 AI 哲学的战略价值。',
                '分析表明：如果执行到位，Poiesis 有望在 2027-2028 年定义一个全新品类——架构优先的可视化开发平台（Architecture-First Visual Development Platform），并整合当前分离的设计、架构、编码三个环节。',
                '关键词：Poiesis；产品愿景；AI 架构助手；多目标编译；竞争格局演化；战略路线图',
            ],
            'chart_keys': DOC2_CHARTS,
        },
        {
            'md_path': BASE_DIR / 'poiesis-value-assessment-2026-07-24.md',
            'output_path': BASE_DIR / 'Poiesis_多维度价值评估_2026.docx',
            'title': 'Poiesis 多维度价值评估',
            'subtitle': '结合市场调研与多维度情景分析的综合评估（2026年7月）',
            'date': '2026年7月24日',
            'abstract_lines': [
                '本报告从财务价值、个人发展价值、群体/生态价值、用户规模等多个维度，对 Poiesis 项目进行系统性价值评估。',
                '采用情景分析法，设定了保守、中等、乐观、极乐观四种财务情景，并基于概率加权计算了项目期望价值。同时，引入"用户规模-价值弹性"分析框架，论证了个人价值（技能积累、品牌建设、认知能力提升）在极小用户规模时即可达到饱和的结论。',
                '核心发现：即使在最悲观情景（零用户、零收入）下，Poiesis 为开发者带来的能力积累——包括架构决策与系统思维、AI 编排与指挥、产品品味与设计判断等——其职业溢价在 3-5 年内预计可达 50万-200万元人民币，相对约5000元的 AI 开发费用，投资回报率超过 100 倍。',
                '关键词：Poiesis；价值评估；情景分析；投资回报率；个人发展；AI 时代职业能力',
            ],
            'chart_keys': DOC3_CHARTS,
        },
    ]

    for config in doc_configs:
        print(f"\n  处理: {config['md_path'].name}")
        build_document(
            md_path=config['md_path'],
            output_path=config['output_path'],
            title=config['title'],
            subtitle=config['subtitle'],
            date=config['date'],
            abstract_lines=config['abstract_lines'],
            chart_keys=config['chart_keys'],
            chart_dir=CHARTS_DIR,
        )

    print("\n" + "=" * 60)
    print("转换完成！生成的文件：")
    for config in doc_configs:
        size_kb = config['output_path'].stat().st_size / 1024
        print(f"  📄 {config['output_path'].name} ({size_kb:.1f} KB)")
    print(f"\n图表文件位于: {CHARTS_DIR}/")
    print("=" * 60)


if __name__ == '__main__':
    main()
