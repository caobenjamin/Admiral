
我们先从服务端的代码开始进行。


1，用户登录、注册和鉴权服务。请按照《Admiral_v1_Architecture.md》中的“8. MVP 认证流程（简化版）”进行实现。

2，广告平台的授权部分。

2.1 MAPI OAuth2.0授权服务。

用户在客户端点击“授权”后，先向服务端申请跳转链接。服务端确认用户的登录信息后，生成跳转链接，并返回给客户端。

生成的跳转连接如下：
https://ad.bilibili.com/developer/developer-authorization?client_id={APP_ID}&state={STATE}

其中：APP_ID为广告平台的APP_ID，需要在服务端进行初始化；STATE为随机生成的状态码，与用户信息进行匹配。

用户在浏览器中打开跳转链接，进行授权。授权成功后，浏览器会自动跳转到CALLBACK_URL，并携带code参数和state参数。

服务端收到code和state参数后，调用接口：

curl --location --request POST 'https://cm.bilibili.com/takumi/api/open_api/oauth2/token' \
--data-urlencode 'grant_type=authorization_code' \
--data-urlencode 'code=code_参数' \
--data-urlencode 'client_id=APP_ID' \
--data-urlencode 'client_secret=APP_SECRET' \
--data-urlencode 'redirect_uri=CALLBACK_URL'

其中：APP_ID为广告平台的APP_ID，需要在服务端进行初始化；APP_SECRET为广告平台的APP_SECRET，需要在服务端进行初始化；CALLBACK_URL为回调地址，需要在服务端进行初始化。

返回结果如下：
{
    "message": "success",
    "code": 0,
    "data": {
        "access_token": "access_token_gx1LZnmc0C5m4I2ypH-RfpcsqGiRKcih-5lnOwB_WeM",
        "refresh_token": "refresh_token_3NYMj1pQT0dNg_RbpZAZJV_XmFzvUO_KVwV1C5VK43M",
        "token_type": "Bearer",
        "expires_in": 86400
    }
}

服务端收到返回的token和refreshtoken后，根据state参数辨别用户信息，将token存储在数据库中，并告知客户端授权成功。（注意：1个用户可以进行多次授权，获取到多个token，需要进行区分）

同时服务端需要运行token的自刷新服务，每隔10个小时就要完成一次token的刷新。

刷新token的接口如下：

curl --location --request POST 'https://cm.bilibili.com/takumi/api/open_api/oauth2/token' \
--data-urlencode 'grant_type=refresh_token' \
--data-urlencode 'client_id=APP_ID' \
--data-urlencode 'client_secret=APP_SECRET' \
--data-urlencode 'refresh_token=refresh_token_参数'

其中：APP_ID为广告平台的APP_ID，需要在服务端进行初始化；APP_SECRET为广告平台的APP_SECRET，需要在服务端进行初始化；refresh_token来源于数据库中存储的refresh_token。

获取返回信息：{
    "message": "success",
    "code": 0,
    "data": {
        "access_token": "access_token_7GB2vDdc__kCeiw_9QE4e8H-_kn4-YnpIeoiRhMpKKo",
        "refresh_token": "refresh_token_3NYMj1pQT0dNg_RbpZAZJV_XmFzvUO_KVwV1C5VK43M",
        "token_type": "Bearer",
        "expires_in": 86400
    }
}

2.2 广告账户绑定。

用户在客户端点击“绑定账户”后，服务端调用广告平台的账户列表接口，获取到可以管理的账户信息，展示在页面上。

curl  --request GET 'https://cm.bilibili.com/takumi/api/open_api/oauth2/advertisers?page=1&size=100' \
--header 'Authorization: Bearer YOUR_YOUR_ACCESS_TOKEN' 

{
    "message": "success",
    "code": 0,
    "data": {
        "records": [
            {
                "account_id": 431037,
                "account_name": "起飞测试11",
                "account_role": 2
            }
        ],
        "total": 1,
        "page": 1,
        "size": 100
    }
}

用户点击多选账户后，点击确定后，正式建立 用户 - 账户 - token/refresh token的绑定关系，并写入数据表。

2.3 MAPI的接口转发服务。

收到客户端的调用请求后，请求内容不变。只进行token的注入和URL的拼接。

URL 为：https://cm.bilibili.com/takumi/api/{接口路径}

header 为：Authorization: Bearer {access_token}

如何匹配token：从请求体中寻找account_id，根据account_id和用户的绑定信息，寻找可以使用的token。


3、大模型调用

将用户的请求转发给大模型，大模型返回结果后，将结果返回给客户端。

请支持以下大模型：（通过官方api进行调用）
- deepseek
- doubao
- chatgpt
- Gemini

初期先不做模型的路由，由管理员在服务端进行配置。
需要相关apikey的，请在服务端进行配置。