# Zix

明示的に登録したproviderを `zix <provider> ...` で呼び出すLinux/WSL向けCLIです。
Google固有の処理は別配布のGoogle extensionが持ち、共通CLIへAPIを集約しません。

[インストールとGoogle IAMの使用例（英語）](docs/install.md)。最初の版は読み取り操作・Linux native実行ファイルに限定します。
OAuthログインや全providerの共通運用が完成したとは扱いません。認証情報は利用者が承認した経路から短命access tokenとして環境変数へ渡します。
