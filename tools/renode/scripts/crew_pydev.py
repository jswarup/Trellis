# tools/renode/scripts/crew_pydev.py
import clr

clr.AddReference("System.Net.Sockets")

from System import Environment
from System.IO import BinaryReader, BinaryWriter
from System.Net.Sockets import TcpClient


def read_response():
    action = reader.ReadInt32()
    addr_lo = reader.ReadUInt32()
    addr_hi = reader.ReadUInt32()
    value_lo = reader.ReadUInt32()
    value_hi = reader.ReadUInt32()
    peripheral_index = reader.ReadInt32()
    return action, addr_lo, addr_hi, value_lo, value_hi, peripheral_index


if request.IsInit:
    node_id = int(Environment.GetEnvironmentVariable("CREW_NODE_ID") or "0")
    base_addr = int(Environment.GetEnvironmentVariable("CREW_BASE_ADDR") or "0x50000000", 0)
    port_str = Environment.GetEnvironmentVariable("CREW_SOCKET_PORT")

    if not port_str:
        raise Exception("CREW_SOCKET_PORT is required for Trellis Crew co-simulation")

    client = TcpClient("127.0.0.1", int(port_str))
    stream = client.GetStream()
    writer = BinaryWriter(stream)
    reader = BinaryReader(stream)
    self.InfoLog("Crew PyDev node %d connected to Trellis on port %s" % (node_id, port_str))
else:
    if 'writer' not in dir() or writer is None or reader is None:
        raise Exception("Crew PyDev access attempted without an active Trellis connection")

    type_str = str(request.Type).upper()
    addr = base_addr + request.Offset

    try:
        if "READ" in type_str:
            writer.Write(23)  # CoSimAction::ReadBusDword
            writer.Write(int(addr & 0xFFFFFFFF))
            writer.Write(int(addr >> 32))
            writer.Write(0)
            writer.Write(0)
            writer.Write(0)
            writer.Flush()
            _, _, _, value_lo, _, _ = read_response()
            request.Value = value_lo
        elif "WRITE" in type_str:
            value = request.Value
            writer.Write(27)  # CoSimAction::WriteBusDword
            writer.Write(int(addr & 0xFFFFFFFF))
            writer.Write(int(addr >> 32))
            writer.Write(int(value & 0xFFFFFFFF))
            writer.Write(int(value >> 32))
            writer.Write(0)
            writer.Flush()
            read_response()
    except Exception as error:
        self.ErrorLog("Crew PyDev socket communication error: %s" % str(error))
        raise
