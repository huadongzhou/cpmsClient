## 架构设计
通过tauri框架的基本能力为基座  添加专属的客户端能力：
- 桌面通知
- 托盘图标
- 自启动
- 网络检测
- 服务请求
- 客户端与视图端事件通信  交互指令/客户端方法/请求代理

入口事件：
- 客户端：获取iframe容器地址
- 视图端：获取客户端返回的iframe容器地址，根据iframe容器地址渲染业务页面

常驻事件：
- 获取本地websocket端口  
1.检测cpms客户端是否存在  
2.获取cpms客户端下的config.conf文件 
3.获取其中的websocket端口信息
- 连接本地websocket服务
- 监听本地websocket服务推送的任务
1.解析任务消息  拿到任务文件路径
2.通过客户端的上传接口  携带iframe实例内的token  转发任务

cpms客户端名称：PrintClient

## 客户端设计
客户端：
1. 主窗体：无头窗口由视图端二次开发
2. 桌面通知： 
- 布局：400*400窗口 屏幕右下角 上headerbar+通知内容
  - headerbar：标题+关闭按钮
  - 通知内容：通知消息
- 流程：
  - 默认加载通知窗口但不显示  只有监听到事件消息时才显示   
  - 监听事件消息  当有事件消息时  显示桌面通知窗口  并渲染通知内容
  - 点击关闭按钮  隐藏桌面通知窗口 
- 约束：
  - 通知窗口只能显示一个  不能同时显示多个通知窗口
3. 托盘：
- 流程：
  - 客户端启动后  创建托盘图标  通过托盘图标点击显示/隐藏客户端窗口
  - 右键点击托盘图标  显示托盘菜单  包含显示/隐藏客户端窗口、启用/禁用自启动、退出应用 
4. 请求：
- 视图端请求客户端
  - 获取iframe容器地址
  - 获取作业列表
  - 获取打印机列表
  - 选择打印机
- 客户端请求服务端
  - 获取iframe容器地址
  - 获取作业列表
  - 获取打印机列表
  - 转发打印任务
- 客户端链接socket
  - 监听推送的打印任务
6. 通信：
- 视图端向客户端发送事件
  - 固定按钮事件：固定/取消固定客户端窗口
  - 收起按钮事件：收起客户端窗口
  - 全屏按钮事件：全屏/退出全屏客户端窗口
  - 关闭按钮事件：关闭客户端窗口
  - 作业列表事件：获取作业列表
  - 打印机列表事件：获取打印机列表
  - 选择打印机事件：获取选择打印机事件
  - 更新token事件：登录后推送token
- 客户端向视图端发送事件
  - 查询token事件：获取iframe实例内的token

视图端：
1. Layout布局容器
  1.1 客户端headbar 功能点：
    1.1.1  标题：logo+title
    1.1.2  按钮：固定按钮+收起按钮+全屏按钮+关闭按钮
  1.2 iframe容器
    1.1.1 postMessage监听公共事件  
    - token：获取iframe实例内的token
  1.3 调试按钮-抽屉弹窗  
    1.3.1 客户端能力状态
    1.3.2 调试客户端能力
    1.3.3 客户端日志查看

## 项目需求

### 需求 1：渲染线上 iframe 容器 地址

客户端启动后请求线上服务，获取 iframe 容器地址，视图端根据返回地址渲染业务页面。

流程：

```text
启动客户端
  -> 客户端请求 服务端获取iframe容器地址
  -> 获取 iframe URL
  -> 缓存 iframe URL
  -> 视图端主动发送事件给客户端获取 iframe URL 并根据 iframe URL 渲染业务页面
```

### 需求 2：连接本地 socket 服务

客户端启动后连接本地 socket 服务，等待任务推送， 

流程：

```text
启动客户端
  -> 检测本地cpms客户端 是否存在
  -> 如果存在  则获取cpms客户端下的DriverClient.ini文件 其中的websocket端口信息 
  -> 根据websocket端口信息  连接本地 socket 服务
  -> 等待任务推送
  -> 解析任务消息
  -> 拿到任务文件路径
  -> 二次转发任务消息  
```

### 需求 3：Token 机制
获取方式：
1. 登录后视图端发送事件推送token
2. 客户端主动发送事件  从iframe实例内获取token

使用方式：
1. 获取到Token时  客户端本地缓存token
2. 客户端与服务端通信  header中携带token
  - 请求失败  清理缓存Token  并主动获取一次新的token 如果token不一致  则重新请求任务 
 
## 项目接口

### socket 推送任务接口 

#### 响应体
"{\n\t\"filePath\":\t\"C:\\\\temp\\\\printer_2_277525937_2_df3bd995_0000fc20_0000468c_0326d710.pdf\",\n\t\"printProperties\":\t{\n\t\t\"uuid\":\t\"f833fb7f05e54ce1800ee5ae44b8258f\",\n\t\t\"printName\":\t\"INSOLU_PRINT_CPMS\",\n\t\t\"driverName\":\t\"Insolu General PDF\",\n\t\t\"realDriverName\":\t\"Insolu General PS\",\n\t\t\"documentName\":\t\"测试页\",\n\t\t\"sourceFileName\":\t\"测试页\",\n\t\t\"hostName\":\t\"DESKTOP-QID169O\",\n\t\t\"machineName\":\t\"\\\\\\\\DESKTOP-QID169O\",\n\t\t\"printPortName\":\t\"cpmsport001port1\",\n\t\t\"portShared\":\t\"0\",\n\t\t\"pageCount\":\t\"1\",\n\t\t\"copyCount\":\t\"1\",\n\t\t\"paper\":\t\"ISOA4\",\n\t\t\"paperSizeValue\":\t\"9\",\n\t\t\"paperWidth\":\t\"2100\",\n\t\t\"paperLength\":\t\"2970\",\n\t\t\"duplexing\":\t\"OneSided\",\n\t\t\"color\":\t\"Color\",\n\t\t\"pageOrientation\":\t\"Portrait\",\n\t\t\"documentCollate\":\t\"Collate\",\n\t\t\"inputSlot\":\t\"Auto\",\n\t\t\"defaultSource\":\t\"Auto\",\n\t\t\"dmDefaultSource\":\t\"15\",\n\t\t\"isPSDriver\":\t\"false\",\n\t\t\"driverBrand\":\t\"pdf\",\n\t\t\"clientIp\":\t\"192.168.99.78\",\n\t\t\"terminalType\":\t\"Windows\",\n\t\t\"printProcessor\":\t\"winprint\",\n\t\t\"printNotifyName\":\t\"Administrator\",\n\t\t\"userName\":\t\"Administrator\",\n\t\t\"printUserName\":\t\"Administrator\",\n\t\t\"DocumentCenterBindingEnable\":\t\"Disable\",\n\t\t\"DocumentBindingEnable\":\t\"Disable\",\n\t\t\"DocumentBindingIndex\":\t\"Left\",\n\t\t\"DocumentBoreEnable\":\t\"Disable\",\n\t\t\"DocumentBoreIndex\":\t\"2HoleLeft\",\n\t\t\"DocumentFoldEnable\":\t\"Disable\",\n\t\t\"DocumentFoldIndex\":\t\"FoldPerJob\",\n\t\t\"PageBooklet\":\t\"None\",\n\t\t\"printArgs\":\t\"[PrintInfo]\\r\\nUUID=printer_2_277525937_2_df3bd995_0000fc20_0000468c_0326d710\\r\\nLocalLang=9\\r\\nLocalCode=UTF16LE\\r\\nExecName=PrintClient.exe\\r\\nDataOutputModel=0\\r\\nColor=2\\r\\nTotalPages=1\\r\\nPagesPrinted=0\\r\\nDuplex=1\\r\\nPaperSize=9\\r\\nPaperName=A4\\r\\nPaperWidth=2100\\r\\nPaperLength=2970\\r\\nOrientation=1\\r\\nCollate=1\\r\\nCopies=1\\r\\nDefaultSource=15\\r\\nPrintPortName=cpmsport001port1\\r\\nPrintName=INSOLU_PRINT_CPMS\\r\\nDriverName=Insolu General PS\\r\\nDocument=测试页\\r\\nNotifyName=Administrator\\r\\nPrintProcessor=winprint\\r\\nMachineName=\\\\\\\\DESKTOP-QID169O\\r\\nUserName=Administrator\\r\\nPrintUserName=Administrator\\r\\nRawPath=C:\\\\temp\\\\printer_2_277525937_2_df3bd995_0000fc20_0000468c_0326d710\\r\\n\\r\\n[GenericInfo]\\r\\nColor=2\\r\\nTotalPages=1\\r\\nDuplex=1\\r\\nPaperSize=9\\r\\nOrientation=1\\r\\nCollate=1\\r\\nCopies=1\\r\\nDefaultSource=15\\r\\n\"\n\t}\n}"

### 作业列表接口

#### 响应体

### 设备列表接口

#### 响应体

### 选择机器接口

#### 响应体

### 转发任务

#### 响应体

