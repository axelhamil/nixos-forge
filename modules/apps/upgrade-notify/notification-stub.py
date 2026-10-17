import asyncio

from dbus_next.aio import MessageBus
from dbus_next.service import ServiceInterface, method


class Notifications(ServiceInterface):
    def __init__(self):
        super().__init__("org.freedesktop.Notifications")

    @method()
    def Notify(self, app: "s", replaces: "u", icon: "s", summary: "s", body: "s", actions: "as", hints: "a{sv}", timeout: "i") -> "u":
        return 1

    @method()
    def GetCapabilities(self) -> "as":
        return ["body"]

    @method()
    def GetServerInformation(self) -> "ssss":
        return ["stub", "test", "1", "1.2"]


async def main():
    bus = await MessageBus().connect()
    bus.export("/org/freedesktop/Notifications", Notifications())
    await bus.request_name("org.freedesktop.Notifications")
    await bus.wait_for_disconnect()


asyncio.run(main())
