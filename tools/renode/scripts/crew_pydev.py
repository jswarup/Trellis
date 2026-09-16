# tools/renode/scripts/crew_pydev.py
import clr
clr.AddReference("System.Net.Sockets")
from System import Environment
from System.Net.Sockets import TcpClient
from System.IO import BinaryReader, BinaryWriter

if request.IsInit:
    node_id = 0
    client = None
    writer = None
    reader = None
    port_str = Environment.GetEnvironmentVariable("CREW_SOCKET_PORT")
    if port_str:
        try:
            port = int(port_str)
            client = TcpClient("127.0.0.1", port)
            stream = client.GetStream()
            writer = BinaryWriter(stream)
            reader = BinaryReader(stream)
            self.InfoLog("Crew PyDev connected to Segue on port %d" % port)
        except Exception as e:
            self.WarningLog("Crew PyDev failed to connect to Segue: %s" % str(e))
else:
    type_str = str(request.Type).upper()
    offset = request.Offset
    addr = 0x50000000 + offset

    if writer is not None and reader is not None:
        try:
            if "READ" in type_str:
                action = 23  # CoSimAction::ReadBusDword
                writer.Write(int(action))
                writer.Write(int(addr & 0xFFFFFFFF))
                writer.Write(int(addr >> 32))
                writer.Write(int(0))
                writer.Write(int(0))
                writer.Write(int(0))  # PeripheralIndex
                writer.Flush()

                resp_action = reader.ReadInt32()
                resp_addr_lo = reader.ReadUInt32()
                resp_addr_hi = reader.ReadUInt32()
                resp_val_lo = reader.ReadUInt32()
                resp_val_hi = reader.ReadUInt32()
                resp_periph = reader.ReadInt32()
                request.Value = resp_val_lo
            elif "WRITE" in type_str:
                action = 27  # CoSimAction::WriteBusDword
                val = request.Value
                writer.Write(int(action))
                writer.Write(int(addr & 0xFFFFFFFF))
                writer.Write(int(addr >> 32))
                writer.Write(int(val & 0xFFFFFFFF))
                writer.Write(int(val >> 32))
                writer.Write(int(0))  # PeripheralIndex
                writer.Flush()

                resp_action = reader.ReadInt32()
                resp_addr_lo = reader.ReadUInt32()
                resp_addr_hi = reader.ReadUInt32()
                resp_val_lo = reader.ReadUInt32()
                resp_val_hi = reader.ReadUInt32()
                resp_periph = reader.ReadInt32()
        except Exception as e:
            self.WarningLog("Crew PyDev socket communication error: %s" % str(e))
    else:
        # Standalone mock fallback
        if "READ" in type_str:
            if offset == 0x00:
                request.Value = node_id
            elif offset == 0x04:
                request.Value = 1 | 4  # STATUS_TX_READY | STATUS_PEER_UP
            else:
                request.Value = 0
        elif "WRITE" in type_str:
            if offset == 0x08:
                b = request.Value & 0xFF
                self.InfoLog("Crew MMIO TX (local): 0x%02x ('%s')" % (b, chr(b) if 32 <= b <= 126 else '.'))

