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

以下接口所有 CPMS 请求默认携带：

- `Authorization`：  token，需要鉴权接口携带。
- `Content-Type`：`application/x-www-form-urlencoded`，文件上传为 `multipart/form-data`。
- `access_sign`：按 CPMS 签名规则生成。
- `client=client`
- `platform=harmony`

#### 响应体

普通作业列表：

- 请求方式：POST
- 请求路径：`/cpms/api/jobs/list`
- 请求类型：`application/x-www-form-urlencoded`
- 请求参数：
  - `pageNumber`：当前页码。
  - `pageSize`：每页数量。
  - `type`：作业类型，`1` 打印、`2` 复印、`3` 扫描。
  - `title`：作业标题，默认为空字符串。
  - `searchTime`：查询范围，`now` 今日作业、`history` 历史作业、空字符串为默认范围。

```json
{
  "code": 200,
  "msg": "操作成功",
  "data": {
    "records": [
      {
        "id": "job-001",
        "documentName": "测试页.pdf",
        "commitCount": 1,
        "color": "Color",
        "duplex": "OneSided",
        "jobStatus": 2,
        "jobStatusName": "待打印",
        "compileTime": "2026-06-08 10:00:00",
        "jobPageNum": 1,
        "outDeviceName": "一楼大厅打印机",
        "outDeviceSite": "一楼大厅"
      }
    ],
    "total": 1,
    "size": 20,
    "current": 1
  }
}
```

字段说明：
- `data.records`：普通作业分页数组。
- `data.records[].id`：作业 ID，客户端兼容字符串与数字类型。
- `jobStatus == 2`：待打印状态。

### 设备列表接口

#### 响应体

- 请求方式：GET
- 请求路径：`/cpms/api/userManager/listAvailDevices`
- 说明：获取当前用户可用的授权直连打印设备列表。

```json
{
  "code": 200,
  "msg": "操作成功",
  "data": [
    {
      "deviceId": "device-001",
      "deviceName": "一楼大厅打印机",
      "deviceIp": "192.168.1.120",
      "deviceAuthenticate": "已认证",
      "authType": 1
    }
  ]
}
```

字段说明：
- `deviceId`：设备 ID，选择机器后会作为 `directDeviceId` 参与任务转发。
- `deviceName`：设备名称。
- `deviceIp`：设备 IP。
- `deviceAuthenticate`：设备认证状态，可选。
- `authType`：认证类型，可选。

### 选择机器接口

#### 响应体

选择机器包含服务端更新和客户端本地持久化两步。

服务端更新：

- 请求方式：POST
- 请求路径：`/cpms/api/userManager/updateDirectDeviceId`
- 请求类型：`application/x-www-form-urlencoded`
- 请求参数：
  - `deviceId`：选择的直连打印设备 ID。

```json
{
  "code": 200,
  "msg": "操作成功",
  "data": true
}
```

客户端本地持久化：

```json
{
  "success": true,
  "code": "OK",
  "message": "success",
  "data": {
    "deviceId": "device-001",
    "deviceName": "一楼大厅打印机",
    "deviceIp": "192.168.1.120",
    "deviceAuthenticate": "已认证",
    "authType": 1
  },
  "logs": []
}
```

字段说明：
- `success`：是否保存成功。
- `data`：本次选择的直连设备。
- `logs`：客户端命令日志，默认空数组。

### 转发任务

客户端通过打印任务文件和打印参数转发任务到线上服务。

旧流程：

- 请求方式：POST
- 请求路径：`/cpms/api/jobs/uploadJobByWebOrH5`
- 请求类型：`multipart/form-data`
- 文件字段：`file`

当前主要流程：

- 请求方式：POST
- 请求路径：`/cpms/api/jobs/xps/exec`
- 请求类型：`multipart/form-data`
- 文件字段：`file`
- Query 参数：
  - `fileSuffix`：文件后缀，当前为 `pdf`。
  - `driverType`：驱动类型，当前为 `pdf`。
  - `clientIp`：客户端 IP。
  - `printProperties.driverName`
  - `printProperties.portShared`
  - `printProperties.terminalType`
  - `printProperties.pageCount`
  - `printProperties.copyCount`
  - `printProperties.paper`
  - `printProperties.duplexing`
  - `printProperties.color`
  - `printProperties.pageOrientation`
  - `printProperties.documentCollate`
  - `printProperties.isPSDriver`
  - `title`
  - `printProperties.documentName`
  - `directDeviceId`：用户已选择直连设备时携带。
  - `productType`：产品类型。

#### 响应体

```json
{
  "code": 200,
  "message": "success",
  "data": null
}
```

字段说明：
- `code`：服务端业务状态码，`200` 表示成功。
- `message`：服务端提示信息。
- `data`：转发结果数据，无额外数据时为 `null`。