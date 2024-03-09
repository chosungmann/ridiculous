# Ridiculous

### How to Run

1. Go to https://ridibooks.com/account/login and log in.

2. Open https://account.ridibooks.com/api/user-devices/app to get the device information. Then, in the JSON result to this, find and write down the values of the fields `device_id` and `user_idx`.

    ```json
    {
      "user_devices": [
        {
          "id": 12345678,
          "user_idx": 12345678,
          "device_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
          "device_code": "PC",
          "device_ver": null,
          "device_nick": "PC",
          "status": "enable",
          "last_used": "2024-01-01T00:00:00+09:00",
          "created": "2024-01-01T00:00:00+09:00",
          "last_modified": "2024-01-01T00:00:00+09:00"
        },

        ...

      ]
    }
    ```

3. Clone, build, and run `ridiculous` with these two values.

    ```console
    $ clone https://github.com/chosungmann/ridiculous.git
    $ cd ridiculous
    $ cargo run -- --device-id=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx --user-idx=12345678
    ```

### References

* https://github.com/disjukr/ridi-drm-remover
* https://github.com/sidsidanf/ridi-decrypt
* https://www.bpak.org/blog/2018/04/리디북스-자신이-소유한-책-drm-해제하기-feat-위험한-비밀/
* https://www.bpak.org/blog/2019/05/리디북스-drm-해제하기-한번-더-ft-riberty/
