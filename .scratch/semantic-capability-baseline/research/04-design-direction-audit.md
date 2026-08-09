# 04 — 设计方向「真人写法」审计

**日期**: 2026-08-09
**标准**: 编译器要生成「真人写这个功能时会写的代码」（用户 2026-08-09 拍板测试——错误边界裁决时确立）
**范围**: 已实现域（域 A @form/@load + 既有机制）+ 契约草案（2026-08-08 §4.2/4.3/4.4）

## 审计方法

对每个设计方向问：三目标下「真人（熟练开发者）写这个功能时写出的代码」是什么形态？编译器生成的是否与之等价或同构？

## A. 已实现域（达标检查）

| 域 | React 生成 | 真人写法对照 | Flutter 生成 | 真人写法对照 | 判定 |
|---|---|---|---|---|---|
| @form 校验/提交 | 受控 useState + validateForm 执行器（FormPage.tsx 实证） | 受控组件+手写校验 = 不用库的真实惯用法（react-hook-form 是另一选择，不引库写法同样地道） | Form + TextFormField.validator + GlobalKey.validate() | **Flutter 官方地道写法**（validator 链） | ✅ 达标 |
| @load 三态 | useState 三态 + useEffect 装载 | 手写数据获取惯用法（TanStack Query 是库路线，手写三态是原始路线——真人两者都写） | FutureProvider + AsyncValue（isLoading/asData） | **Riverpod 官方地道** | ✅ 达标 |
| @drag 位移 | pointer props + setPointerCapture | 手写拖拽惯用法（React 无官方 DnD；dnd-kit 是排序生态，位移拖拽手写 pointer 是地道路径） | GestureDetector pan | **官方地道** | ✅ 达标 |
| @ref 直写 | useRef + DOM 直写 | **官方惯用法** | ValueNotifier + GlobalKey | **官方惯用法** | ✅ 达标 |
| 语义事件 14 族 | onClick/onKeyDown… | 官方合成事件 | onTap/onKeyEvent… | 官方 | ✅ 达标 |

**结论：已实现域全部达标**——生成代码与真人惯用写法同构（个别 React 端选「不引库的原始写法」是真人写法的一个子集，可接受；引库是另一条真人路径，非必选）。

## B. 契约草案（达标检查 + 细化点）

| 域 | 草案方向 | React 真人写法 | Flutter 真人写法 | 判定 | 细化点 |
|---|---|---|---|---|---|
| @modal（§4.2） | `<dialog @modal={openSig}>` → React `<dialog>.showModal()` | React 团队对 `<dialog>` 有受控/命令式之争：`showModal()` 命令式 vs `open` prop 受控。真人常用 headless 库或自写 portal+overlay；原生 `<dialog>` 是平台标准（top layer/Esc/焦点内建），MDN 推荐 | showDialog + FocusTraversalGroup = 官方地道 | ✅ 方向达标 | **阶段 spec 细化：React 端受控形态**——生成 `open` 受控 prop（React 惯用法）而非裸 showModal 命令式；锁定时注意 `showModal` 的焦点/Esc 由平台承担 |
| showToast（§4.2） | 内建 showToast → React 自生成轻量 toast | React 无官方 toast：真人要么引库（Sonner/Radix）要么自写轻量实现（容器+定时+堆叠） | ScaffoldMessenger.showSnackBar = 官方地道 | ✅ 达标 | 自生成=「不引依赖的真人手写路径」，YAGNI 合理；堆叠/动画细节阶段 spec 定 |
| @route/navigate/back（§4.3） | @route 声明 + navigate/back 内建 → React Router / Flutter Navigator | 真人 React = React Router（事实标准库） | 真人 Flutter = GoRouter（官方推荐）或 Navigator.push | ✅ 方向达标 | **阶段 spec 细化：库绑定与依赖注入**——生成代码引第三方库有先例（Zustand，design.rs:556 实证）；React Router 绑定方式（依赖版本/引入形态）与 Flutter GoRouter vs Navigator 取舍在域 C spec 定 |
| bp("md") 断点（§4.4） | breakpoint token + bp() 条件 → React matchMedia / Flutter MediaQuery | 逻辑断点真人写 useMediaQuery/matchMedia hook | MediaQuery = 官方地道 | ✅ 达标 | 无 |

**结论：契约草案方向全部达标**，3 个细化点登记（@modal React 受控形态 / @route 库绑定与依赖注入 / showToast 自生成细节）——均属阶段 spec 决策，不阻塞方向级裁决。

## 审计原则沉淀

「真人写法」测试的补充判据：
- **引库路线不构成否决**：React 生态事实标准库（React Router/TanStack Query）是真人写法的主流路线——编译器生成「事实标准库调用」是达标的；「自生成原始写法」也是真人写法的合法子集（零依赖偏好）
- **跨目标差异的真实解法**：某一端无地道形态 → 该意图的语言化需降级路径或留生态（错误边界先例）
- 已实现域作为「达标基准」：@form/@load/@drag/@ref 的生成形态即「地道代码」的事实样本，新域方向以此为参照系
